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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Invitation, Membership};
    use audit_log::models::{AuditOutcome, AuditSeverity};
    use auth::public::AuthUserId;
    use chrono::TimeZone;
    use platform_core::{ActorContext, CorrelationId, RequestId};

    #[test]
    fn audit_adapter_builds_expected_event_mappings() {
        let now = Utc.with_ymd_and_hms(2026, 7, 5, 12, 0, 0).unwrap();
        let mut request_ctx = RequestContext::new(
            RequestId::new("req_audit"),
            CorrelationId::new("corr_audit"),
        );
        request_ctx.actor = ActorContext::User {
            user_id: "usr_owner".to_owned(),
            scopes: Vec::new(),
        };
        let organization = Organization {
            id: "org_1".to_owned(),
            name: "Acme".to_owned(),
            slug: "acme".to_owned(),
            created_at: now,
            updated_at: now,
            archived_at: None,
        };

        let organization_event = organization_created(&request_ctx, &organization, now);
        assert_eq!(organization_event.event_name, "organization.created");
        assert_eq!(organization_event.module_name, MODULE_NAME);
        assert_eq!(organization_event.action, "created");
        assert_eq!(organization_event.outcome, AuditOutcome::Success);
        assert_eq!(organization_event.severity, AuditSeverity::Info);
        assert_eq!(
            organization_event.scope.expect("organization scope"),
            AuditScope {
                module: Some(MODULE_NAME.to_owned()),
                scope_type: "organization".to_owned(),
                id: "org_1".to_owned(),
                display: Some("Acme".to_owned()),
            }
        );
        assert_eq!(
            organization_event.resource.expect("organization resource"),
            AuditResource {
                resource_type: "organization".to_owned(),
                id: "org_1".to_owned(),
                display: Some("Acme".to_owned()),
            }
        );
        assert_eq!(organization_event.metadata["slug"], "acme");

        let expires_at = now + chrono::Duration::days(1);
        let created = CreatedInvitation {
            invitation: Invitation {
                id: "invite_1".to_owned(),
                organization_id: "org_1".to_owned(),
                email: "member@example.com".to_owned(),
                role_id: "role_member".to_owned(),
                expires_at,
                created_at: now,
                updated_at: now,
                accepted_at: None,
                revoked_at: None,
            },
            token: "raw-token".to_owned(),
        };

        let invitation_event = invitation_created(&request_ctx, &created, now);
        assert_success_context(
            &invitation_event,
            "organization.invitation_created",
            "invitation_created",
            "org_1",
        );
        assert_eq!(
            invitation_event.resource.expect("invitation resource"),
            AuditResource {
                resource_type: "organization_invitation".to_owned(),
                id: "invite_1".to_owned(),
                display: Some("member@example.com".to_owned()),
            }
        );
        assert_eq!(invitation_event.metadata["email"], "member@example.com");
        assert_eq!(invitation_event.metadata["role_id"], "role_member");
        assert_eq!(invitation_event.metadata["expires_at"], json!(expires_at));

        let membership = Membership {
            id: "member_1".to_owned(),
            organization_id: "org_1".to_owned(),
            auth_user_id: AuthUserId("usr_member".to_owned()),
            role_id: "role_member".to_owned(),
            role_name: Some("member".to_owned()),
            created_at: now,
            updated_at: now,
            removed_at: None,
        };

        let accepted_event = invitation_accepted(&request_ctx, &membership, now);
        assert_success_context(
            &accepted_event,
            "organization.invitation_accepted",
            "invitation_accepted",
            "org_1",
        );
        assert_eq!(
            accepted_event.resource.expect("membership resource"),
            AuditResource {
                resource_type: "organization_member".to_owned(),
                id: "member_1".to_owned(),
                display: Some("usr_member".to_owned()),
            }
        );
        assert_eq!(accepted_event.metadata["auth_user_id"], "usr_member");
        assert_eq!(accepted_event.metadata["role_id"], "role_member");
        assert_eq!(accepted_event.metadata["role_name"], "member");
    }

    fn assert_success_context(
        event: &AuditEventInput,
        event_name: &str,
        action: &str,
        scope_id: &str,
    ) {
        assert_eq!(event.event_name, event_name);
        assert_eq!(event.action, action);
        assert_eq!(event.module_name, MODULE_NAME);
        assert_eq!(event.outcome, AuditOutcome::Success);
        assert_eq!(event.severity, AuditSeverity::Info);

        let scope = event.scope.as_ref().expect("organization scope");
        assert_eq!(scope.module.as_deref(), Some(MODULE_NAME));
        assert_eq!(scope.scope_type, "organization");
        assert_eq!(scope.id, scope_id);

        let request = event.request.as_ref().expect("request context");
        assert_eq!(request.correlation_id.as_deref(), Some("corr_audit"));
        assert_eq!(request.request_id.as_deref(), Some("req_audit"));
        assert_eq!(request.story_id.as_deref(), Some("corr_audit"));
    }
}
