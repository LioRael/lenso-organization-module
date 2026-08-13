use crate::models::Invitation;
use crate::repositories::PostgresOrganizationRepository;
use chrono::{DateTime, Utc};
use lenso::host::outbox::ErrorCode;
use lenso::host::transaction::{AppError, AppResult, LinkedTransaction};
use notification::public::{
    CreateTransactionalEmailIntent, EmailRecipient, IntentSource, OrganizationInvitationTemplateV1,
    TransactionalIntentReceipt, create_transactional_email_intent_in_tx_with_protector,
};
use notification::snapshot::{EnvironmentSnapshotProtector, SnapshotProtector};
use platform_core::RequestContext;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use url::Url;

pub const INVITATION_DELIVERY_IDEMPOTENCY_HEADER: &str = "idempotency-key";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateInvitationDelivery {
    pub email: String,
    pub role_id: String,
    pub expires_at: DateTime<Utc>,
    pub locale: String,
    pub recipient_display_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InvitationDeliveryReceipt {
    pub invitation_id: String,
    pub organization_id: String,
    pub notification_intent_id: String,
    pub notification_delivery_id: String,
    pub delivery_status: String,
    pub expires_at: DateTime<Utc>,
    pub idempotent_replay: bool,
}

type InvitationDeliveryRequestRow = (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

pub async fn create_invitation_delivery(
    repository: &PostgresOrganizationRepository,
    request_ctx: &RequestContext,
    organization_id: &str,
    request: &CreateInvitationDelivery,
    idempotency_key: &str,
    public_invitation_base_url: &str,
    now: DateTime<Utc>,
) -> AppResult<InvitationDeliveryReceipt> {
    let protector = EnvironmentSnapshotProtector::from_env()?;
    create_invitation_delivery_with_protector(
        repository,
        request_ctx,
        organization_id,
        request,
        idempotency_key,
        public_invitation_base_url,
        now,
        &protector,
    )
    .await
}

pub async fn create_invitation_delivery_with_protector(
    repository: &PostgresOrganizationRepository,
    request_ctx: &RequestContext,
    organization_id: &str,
    request: &CreateInvitationDelivery,
    idempotency_key: &str,
    public_invitation_base_url: &str,
    now: DateTime<Utc>,
    protector: &dyn SnapshotProtector,
) -> AppResult<InvitationDeliveryReceipt> {
    create_invitation_delivery_in_tx(
        repository,
        request_ctx,
        organization_id,
        request,
        idempotency_key,
        public_invitation_base_url,
        now,
        protector,
    )
    .await
}

async fn create_invitation_delivery_in_tx(
    repository: &PostgresOrganizationRepository,
    request_ctx: &RequestContext,
    organization_id: &str,
    request: &CreateInvitationDelivery,
    idempotency_key: &str,
    public_invitation_base_url: &str,
    now: DateTime<Utc>,
    protector: &dyn SnapshotProtector,
) -> AppResult<InvitationDeliveryReceipt> {
    let idempotency_key = required_bounded(idempotency_key, "Idempotency-Key", 300)?;
    let locale = required_bounded(&request.locale, "locale", 35)?;
    let public_base_url = validate_public_invitation_base_url(public_invitation_base_url)?;
    let normalized_email = normalize_email(&request.email)?;
    if request.expires_at <= now {
        return Err(AppError::new(
            ErrorCode::Validation,
            "expires_at must be in the future",
        ));
    }
    let recipient_display_name = request
        .recipient_display_name
        .as_deref()
        .map(|value| required_bounded(value, "recipient_display_name", 160))
        .transpose()?;
    let digest = request_digest(
        organization_id,
        &normalized_email,
        &request.role_id,
        request.expires_at,
        locale,
        recipient_display_name,
    );

    let mut tx = LinkedTransaction::begin(repository.pool()).await?;
    let claim_id = new_id("org_inv_delivery");
    let inserted = sqlx::query_scalar::<_, String>(
        r#"
        insert into organization.invitation_delivery_requests (
            id, organization_id, idempotency_key, request_digest, created_at
        ) values ($1, $2, $3, $4, $5)
        on conflict (organization_id, idempotency_key) do nothing
        returning id
        "#,
    )
    .bind(&claim_id)
    .bind(organization_id)
    .bind(idempotency_key)
    .bind(&digest)
    .bind(now)
    .fetch_optional(&mut **tx.sql())
    .await
    .map_err(map_sql_error)?;

    if inserted.is_none() {
        let existing = sqlx::query_as::<_, InvitationDeliveryRequestRow>(
            r#"
            select request_digest, invitation_id, notification_intent_id,
                   notification_delivery_id, initial_delivery_status
            from organization.invitation_delivery_requests
            where organization_id = $1 and idempotency_key = $2
            for update
            "#,
        )
        .bind(organization_id)
        .bind(idempotency_key)
        .fetch_one(&mut **tx.sql())
        .await
        .map_err(map_sql_error)?;
        if existing.0 != digest {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "Idempotency-Key was already used for a different invitation delivery request",
            ));
        }
        let Some(receipt) = completed_receipt(existing, organization_id, request.expires_at) else {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "Invitation delivery with this Idempotency-Key is still being created",
            )
            .retryable());
        };
        tx.commit().await?;
        return Ok(receipt);
    }

    let (created, organization_name, role_name) = repository
        .create_invitation_in_tx(
            tx.sql(),
            organization_id,
            &normalized_email,
            &request.role_id,
            request.expires_at,
            now,
        )
        .await?;
    let invitation_url = invitation_url(&public_base_url, &created.token)?;
    let intent = CreateTransactionalEmailIntent {
        source: IntentSource {
            module_id: "organization".to_owned(),
            entity_type: "organization_invitation".to_owned(),
            entity_id: created.invitation.id.clone(),
        },
        recipient: EmailRecipient {
            address: normalized_email,
            display_name: recipient_display_name.map(ToOwned::to_owned),
            locale: locale.to_owned(),
        },
        template: OrganizationInvitationTemplateV1 {
            organization_id: organization_id.to_owned(),
            organization_name,
            invitation_id: created.invitation.id.clone(),
            invitation_url,
            inviter_display_name: None,
            role_name: Some(role_name),
            expires_at: request.expires_at,
        },
        idempotency_key: format!("organization-invitation:{organization_id}:{idempotency_key}"),
        correlation_id: request_ctx.correlation_id.0.clone(),
        causation_id: request_ctx.causation_id.clone(),
        requested_by: request_actor_id(request_ctx),
    };
    let notification =
        create_transactional_email_intent_in_tx_with_protector(&mut tx, &intent, now, protector)
            .await?;

    #[cfg(feature = "audit-log")]
    audit_log::repositories::PostgresAuditLogRepository::new(repository.pool().clone())
        .record_event_in_tx(
            tx.sql(),
            crate::audit::invitation_created(request_ctx, &created, now),
        )
        .await?;

    sqlx::query(
        r#"
        update organization.invitation_delivery_requests
        set invitation_id = $2,
            notification_intent_id = $3,
            notification_delivery_id = $4,
            initial_delivery_status = $5,
            completed_at = $6
        where id = $1
        "#,
    )
    .bind(&claim_id)
    .bind(&created.invitation.id)
    .bind(&notification.intent_id)
    .bind(&notification.delivery_id)
    .bind(notification.status.as_str())
    .bind(now)
    .execute(&mut **tx.sql())
    .await
    .map_err(map_sql_error)?;
    tx.commit().await?;

    Ok(receipt(&created.invitation, notification, false))
}

fn completed_receipt(
    row: InvitationDeliveryRequestRow,
    organization_id: &str,
    expires_at: DateTime<Utc>,
) -> Option<InvitationDeliveryReceipt> {
    Some(InvitationDeliveryReceipt {
        invitation_id: row.1?,
        organization_id: organization_id.to_owned(),
        notification_intent_id: row.2?,
        notification_delivery_id: row.3?,
        delivery_status: row.4?,
        expires_at,
        idempotent_replay: true,
    })
}

fn receipt(
    invitation: &Invitation,
    notification: TransactionalIntentReceipt,
    idempotent_replay: bool,
) -> InvitationDeliveryReceipt {
    InvitationDeliveryReceipt {
        invitation_id: invitation.id.clone(),
        organization_id: invitation.organization_id.clone(),
        notification_intent_id: notification.intent_id,
        notification_delivery_id: notification.delivery_id,
        delivery_status: notification.status.as_str().to_owned(),
        expires_at: invitation.expires_at,
        idempotent_replay,
    }
}

fn validate_public_invitation_base_url(value: &str) -> AppResult<Url> {
    let value = required_bounded(value, "public invitation base URL", 2_048)?;
    let url = Url::parse(value).map_err(|_| {
        AppError::new(
            ErrorCode::Validation,
            "public invitation base URL must be an absolute HTTP(S) URL",
        )
    })?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(AppError::new(
            ErrorCode::Validation,
            "public invitation base URL must be an absolute HTTP(S) URL",
        ));
    }
    if url.username() != ""
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(AppError::new(
            ErrorCode::Validation,
            "public invitation base URL must not contain credentials, query, or fragment",
        ));
    }
    Ok(url)
}

fn invitation_url(base: &Url, token: &str) -> AppResult<String> {
    base.join(&format!("invitations/{token}"))
        .map(String::from)
        .map_err(|_| {
            AppError::new(
                ErrorCode::Validation,
                "public invitation base URL is invalid",
            )
        })
}

fn normalize_email(value: &str) -> AppResult<String> {
    let value = required_bounded(value, "email", 320)?.to_ascii_lowercase();
    let Some((local, domain)) = value.rsplit_once('@') else {
        return Err(AppError::new(ErrorCode::Validation, "email is invalid"));
    };
    if local.is_empty()
        || domain.is_empty()
        || domain.starts_with('.')
        || domain.ends_with('.')
        || !domain.contains('.')
        || value.chars().any(char::is_whitespace)
    {
        return Err(AppError::new(ErrorCode::Validation, "email is invalid"));
    }
    Ok(value)
}

fn required_bounded<'a>(value: &'a str, name: &str, max: usize) -> AppResult<&'a str> {
    let value = value.trim();
    if value.is_empty() || value.len() > max {
        return Err(AppError::new(
            ErrorCode::Validation,
            format!("{name} must contain between 1 and {max} bytes"),
        ));
    }
    Ok(value)
}

fn request_digest(
    organization_id: &str,
    email: &str,
    role_id: &str,
    expires_at: DateTime<Utc>,
    locale: &str,
    recipient_display_name: Option<&str>,
) -> String {
    let canonical = serde_json::json!({
        "email": email,
        "expiresAt": expires_at.to_rfc3339(),
        "locale": locale,
        "organizationId": organization_id,
        "recipientDisplayName": recipient_display_name,
        "roleId": role_id,
    });
    let bytes = serde_json::to_vec(&canonical).expect("invitation delivery request serializes");
    let digest = Sha256::digest(bytes);
    let mut value = String::with_capacity(71);
    value.push_str("sha256:");
    for byte in digest {
        let _ = write!(&mut value, "{byte:02x}");
    }
    value
}

fn request_actor_id(request_ctx: &RequestContext) -> Option<String> {
    match &request_ctx.actor {
        platform_core::ActorContext::User { user_id, .. } => Some(user_id.clone()),
        platform_core::ActorContext::Service { service_id, .. } => Some(service_id.clone()),
        _ => None,
    }
}

fn new_id(prefix: &str) -> String {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("operating system randomness should be available");
    let mut id = String::with_capacity(prefix.len() + 33);
    id.push_str(prefix);
    id.push('_');
    for byte in bytes {
        let _ = write!(&mut id, "{byte:02x}");
    }
    id
}

fn map_sql_error(source: sqlx::Error) -> AppError {
    AppError::new(
        ErrorCode::Internal,
        "Invitation delivery persistence failed",
    )
    .with_source(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_email_and_rejects_unsafe_public_url() {
        assert_eq!(
            normalize_email(" Member@Example.COM ").unwrap(),
            "member@example.com"
        );
        assert!(validate_public_invitation_base_url("https://user:secret@example.com/").is_err());
        assert!(validate_public_invitation_base_url("mailto:member@example.com").is_err());
    }

    #[test]
    fn digest_uses_normalized_stable_fields() {
        let now = DateTime::parse_from_rfc3339("2026-08-13T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            request_digest("org_1", "member@example.com", "role_1", now, "en-US", None),
            request_digest("org_1", "member@example.com", "role_1", now, "en-US", None)
        );
    }
}
