use crate::models::{CreatedInvitation, Membership, Organization};
use crate::module::MODULE_NAME;
use audit_log::models::{AuditEventInput, AuditResource, AuditScope};
use chrono::{DateTime, Utc};
use platform_core::RequestContext;
use serde_json::{Value, json};

pub fn organization_created(
    request_ctx: &RequestContext,
    organization: &Organization,
    now: DateTime<Utc>,
) -> AuditEventInput {
    success_event(
        request_ctx,
        "created",
        Some(organization_scope(
            &organization.id,
            Some(organization.name.clone()),
        )),
        Some(AuditResource {
            resource_type: "organization".to_owned(),
            id: organization.id.clone(),
            display: Some(organization.name.clone()),
        }),
        json!({ "slug": organization.slug }),
        now,
    )
}

pub fn invitation_created(
    request_ctx: &RequestContext,
    created: &CreatedInvitation,
    now: DateTime<Utc>,
) -> AuditEventInput {
    success_event(
        request_ctx,
        "invitation_created",
        Some(organization_scope(
            &created.invitation.organization_id,
            None,
        )),
        Some(AuditResource {
            resource_type: "organization_invitation".to_owned(),
            id: created.invitation.id.clone(),
            display: Some(created.invitation.email.clone()),
        }),
        json!({
            "email": created.invitation.email,
            "role_id": created.invitation.role_id,
            "expires_at": created.invitation.expires_at,
        }),
        now,
    )
}

pub fn invitation_accepted(
    request_ctx: &RequestContext,
    membership: &Membership,
    now: DateTime<Utc>,
) -> AuditEventInput {
    success_event(
        request_ctx,
        "invitation_accepted",
        Some(organization_scope(&membership.organization_id, None)),
        Some(AuditResource {
            resource_type: "organization_member".to_owned(),
            id: membership.id.clone(),
            display: Some(membership.auth_user_id.0.clone()),
        }),
        json!({
            "auth_user_id": membership.auth_user_id.0,
            "role_id": membership.role_id,
            "role_name": membership.role_name,
        }),
        now,
    )
}

fn success_event(
    request_ctx: &RequestContext,
    action: &str,
    scope: Option<AuditScope>,
    resource: Option<AuditResource>,
    metadata: Value,
    now: DateTime<Utc>,
) -> AuditEventInput {
    let mut input = audit_log::public::success_input(
        request_ctx,
        MODULE_NAME,
        action,
        scope,
        resource,
        None,
        metadata,
    );
    input.occurred_at = now;
    input
}

fn organization_scope(id: &str, display: Option<String>) -> AuditScope {
    AuditScope {
        module: Some(MODULE_NAME.to_owned()),
        scope_type: "organization".to_owned(),
        id: id.to_owned(),
        display,
    }
}
