#[cfg(feature = "audit-log")]
use audit_log::migrations::AUDIT_LOG_MIGRATIONS;
#[cfg(feature = "audit-log")]
use audit_log::models::AuditEventFilter;
#[cfg(feature = "audit-log")]
use audit_log::repositories::PostgresAuditLogRepository;
use auth::models::{AuthUser, AuthUserId};
use auth::repositories::{AuthUserRepository, PostgresAuthUserRepository};
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use axum::middleware;
use chrono::{Duration, Utc};
use organization::admin::OrganizationAdminData;
use organization::module::{
    ORGANIZATION_INVITATIONS_MANAGE, ORGANIZATION_MANAGE, ORGANIZATION_READ,
};
use organization::repositories::PostgresOrganizationRepository;
use platform_core::{AppConfig, LoggingEventPublisher, PLATFORM_MIGRATIONS, apply_migrations};
use platform_http::request_context_middleware;
use platform_module::{AdminActionSource, AdminDataSource, AdminListQuery};
use platform_runtime::RUNTIME_MIGRATIONS;
use platform_testing::TestDatabase;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn public_helpers_create_seed_roles_and_check_permissions() {
    let Some(db) = migrated_database().await else {
        return;
    };

    seed_user(&db.pool, "usr_owner").await;
    seed_user(&db.pool, "usr_stranger").await;

    let now = Utc::now();
    let owner = AuthUserId("usr_owner".to_owned());
    let organization =
        organization::public::create_organization_with_owner(&db.pool, "Acme", "acme", &owner, now)
            .await
            .expect("organization created");

    assert!(
        organization::public::has_permission(
            &db.pool,
            &organization.id,
            &owner,
            ORGANIZATION_MANAGE,
        )
        .await
        .expect("owner permission")
    );
    assert!(
        organization::public::has_permission(
            &db.pool,
            &organization.id,
            &owner,
            ORGANIZATION_READ,
        )
        .await
        .expect("owner read permission")
    );
    assert!(
        !organization::public::has_permission(
            &db.pool,
            &organization.id,
            &AuthUserId("usr_stranger".to_owned()),
            ORGANIZATION_MANAGE,
        )
        .await
        .expect("stranger permission")
    );

    let organizations = organization::public::list_user_organizations(&db.pool, &owner)
        .await
        .expect("list organizations");
    assert_eq!(organizations, vec![organization]);

    db.cleanup().await;
}

#[tokio::test]
async fn admin_data_and_actions_hide_token_hash_and_keep_terminal_records_visible() {
    let Some(db) = migrated_database().await else {
        return;
    };

    seed_user(&db.pool, "usr_owner").await;
    seed_user(&db.pool, "usr_invited").await;

    let repo = Arc::new(PostgresOrganizationRepository::new(db.pool.clone()));
    let admin = OrganizationAdminData::new(repo.clone());
    let now = Utc::now();
    let organization = repo
        .create_organization_with_owner(
            "Acme Admin",
            "acme-admin",
            &AuthUserId("usr_owner".to_owned()),
            now,
        )
        .await
        .expect("organization created");
    let member_role = repo
        .member_role_for_organization(&organization.id)
        .await
        .expect("member role");
    let admin_role = repo
        .create_role(
            &organization.id,
            "admin",
            &[ORGANIZATION_MANAGE.to_owned()],
            now,
        )
        .await
        .expect("admin role");

    let created = admin
        .invoke(
            "create_invitation",
            json!({
                "organization_id": organization.id,
                "email": "invited@example.com",
                "role_id": member_role.id,
                "expires_at": (now + Duration::days(1)).to_rfc3339(),
            }),
        )
        .await
        .expect("create invitation action");
    let token = created["token"].as_str().expect("raw token").to_owned();
    assert!(token.starts_with("org_inv_token_"));

    let invitations = admin
        .list("invitations", &AdminListQuery::new(10, None))
        .await
        .expect("list invitations");
    assert_eq!(invitations.records.len(), 1);
    assert_eq!(invitations.records[0]["email"], "invited@example.com");
    assert!(invitations.records[0].get("token").is_none());
    assert!(invitations.records[0].get("token_hash").is_none());

    let membership = organization::public::accept_invitation(
        &db.pool,
        &token,
        &AuthUserId("usr_invited".to_owned()),
        now,
    )
    .await
    .expect("accept invitation");
    assert_eq!(membership.role_name.as_deref(), Some("member"));

    let updated = admin
        .invoke(
            "update_member_role",
            json!({
                "membership_id": membership.id,
                "role_id": admin_role.id,
            }),
        )
        .await
        .expect("update member role");
    assert_eq!(updated["updated"], true);

    assert!(
        repo.has_permission(
            &organization.id,
            &AuthUserId("usr_invited".to_owned()),
            ORGANIZATION_MANAGE,
        )
        .await
        .expect("updated permission")
    );

    let removed = admin
        .invoke("remove_member", json!({ "membership_id": membership.id }))
        .await
        .expect("remove member");
    assert_eq!(removed["removed"], true);

    let memberships = admin
        .list("memberships", &AdminListQuery::new(10, None))
        .await
        .expect("list memberships");
    assert!(
        memberships
            .records
            .iter()
            .any(|record| record["auth_user_id"] == "usr_invited"
                && record["removed_at"].as_str().is_some())
    );

    let second = repo
        .create_invitation(
            &organization.id,
            "revoked@example.com",
            &member_role.id,
            now + Duration::days(1),
            now,
        )
        .await
        .expect("second invitation");
    let revoked = admin
        .invoke(
            "revoke_invitation",
            json!({ "invitation_id": second.invitation.id }),
        )
        .await
        .expect("revoke invitation");
    assert_eq!(revoked["revoked"], true);

    let revoked_record = admin
        .get("invitations", &second.invitation.id)
        .await
        .expect("get revoked invitation")
        .expect("revoked invitation");
    assert!(revoked_record["revoked_at"].as_str().is_some());
    assert!(revoked_record.get("token_hash").is_none());

    db.cleanup().await;
}

#[tokio::test]
async fn member_role_updates_protect_owner_memberships() {
    let Some(db) = migrated_database().await else {
        return;
    };

    seed_user(&db.pool, "usr_owner_protected").await;
    seed_user(&db.pool, "usr_regular_member").await;

    let now = Utc::now();
    let owner = AuthUserId("usr_owner_protected".to_owned());
    let repo = Arc::new(PostgresOrganizationRepository::new(db.pool.clone()));
    let organization = repo
        .create_organization_with_owner("Protected Org", "protected-org", &owner, now)
        .await
        .expect("organization created");
    let member_role = repo
        .member_role_for_organization(&organization.id)
        .await
        .expect("member role");
    let owner_role = repo
        .owner_role_for_organization(&organization.id)
        .await
        .expect("owner role");
    let invitation = repo
        .create_invitation(
            &organization.id,
            "regular@example.com",
            &member_role.id,
            now + Duration::days(1),
            now,
        )
        .await
        .expect("invitation created");
    let member = repo
        .accept_invitation(
            &invitation.token,
            &AuthUserId("usr_regular_member".to_owned()),
            now,
        )
        .await
        .expect("invitation accepted");
    let owner_membership = repo
        .list_members(&organization.id)
        .await
        .expect("members")
        .into_iter()
        .find(|membership| membership.auth_user_id == owner)
        .expect("owner membership");

    let rejected_promotion = repo
        .update_member_role(&member.id, &owner_role.id, now)
        .await
        .expect_err("member management cannot assign owner role");
    assert!(
        rejected_promotion
            .to_string()
            .contains("owner role cannot be assigned")
    );

    let rejected_owner_role_change = repo
        .update_member_role(&owner_membership.id, &member_role.id, now)
        .await
        .expect_err("owner membership role cannot be changed");
    assert!(
        rejected_owner_role_change
            .to_string()
            .contains("owner membership role cannot be changed")
    );

    let rejected_owner_removal = repo
        .remove_member(&owner_membership.id, now)
        .await
        .expect_err("owner membership cannot be removed");
    assert!(
        rejected_owner_removal
            .to_string()
            .contains("owner membership cannot be removed")
    );

    assert!(
        repo.has_permission(&organization.id, &owner, ORGANIZATION_MANAGE)
            .await
            .expect("owner permission remains")
    );

    db.cleanup().await;
}

#[tokio::test]
async fn http_routes_create_list_invite_accept_and_deny_without_actor_scopes() {
    let Some(db) = migrated_database().await else {
        return;
    };

    seed_user(&db.pool, "usr_owner").await;
    seed_user(&db.pool, "usr_member").await;
    seed_user(&db.pool, "usr_stranger").await;

    let app = test_app(&db);
    let created = request_json(
        app.clone(),
        "POST",
        "/v1/organizations",
        Some("Bearer dev-user:usr_owner"),
        Some(json!({ "name": "Route Org", "slug": "route-org" })),
    )
    .await;
    assert_eq!(created.0, StatusCode::OK);
    let organization_id = created.1["id"]
        .as_str()
        .expect("organization id")
        .to_owned();

    let listed = request_json(
        app.clone(),
        "GET",
        "/v1/organizations",
        Some("Bearer dev-user:usr_owner"),
        None,
    )
    .await;
    assert_eq!(listed.0, StatusCode::OK);
    assert_eq!(listed.1["organizations"][0]["id"], organization_id);

    let repo = PostgresOrganizationRepository::new(db.pool.clone());
    let member_role = repo
        .member_role_for_organization(&organization_id)
        .await
        .expect("member role");
    let invited = request_json(
        app.clone(),
        "POST",
        &format!("/v1/organizations/{organization_id}/invitations"),
        Some("Bearer dev-user:usr_owner"),
        Some(json!({
            "email": "route-member@example.com",
            "role_id": member_role.id,
            "expires_at": (Utc::now() + Duration::days(1)).to_rfc3339(),
        })),
    )
    .await;
    assert_eq!(invited.0, StatusCode::OK);
    let token = invited.1["token"]
        .as_str()
        .expect("invite token")
        .to_owned();

    let accepted = request_json(
        app.clone(),
        "POST",
        &format!("/v1/organization-invitations/{token}/accept"),
        Some("Bearer dev-user:usr_member"),
        None,
    )
    .await;
    assert_eq!(accepted.0, StatusCode::OK);
    assert_eq!(accepted.1["membership"]["role_name"], "member");

    let members = request_json(
        app.clone(),
        "GET",
        &format!("/v1/organizations/{organization_id}/members"),
        Some("Bearer dev-user:usr_owner"),
        None,
    )
    .await;
    assert_eq!(members.0, StatusCode::OK);
    assert_eq!(
        members.1["members"]
            .as_array()
            .expect("members array")
            .len(),
        2
    );

    let denied = request_json(
        app,
        "POST",
        &format!("/v1/organizations/{organization_id}/invitations"),
        Some("Bearer dev-user:usr_stranger"),
        Some(json!({
            "email": "denied@example.com",
            "role_id": member_role.id,
            "expires_at": (Utc::now() + Duration::days(1)).to_rfc3339(),
        })),
    )
    .await;
    assert_eq!(denied.0, StatusCode::FORBIDDEN);

    assert!(
        !repo
            .has_permission(
                &organization_id,
                &AuthUserId("usr_member".to_owned()),
                ORGANIZATION_INVITATIONS_MANAGE,
            )
            .await
            .expect("member invitation permission")
    );

    db.cleanup().await;
}

#[cfg(feature = "audit-log")]
#[tokio::test]
async fn http_routes_write_audit_events_when_audit_feature_is_enabled() {
    let _ = PostgresOrganizationRepository::create_organization_with_owner_audited;
    let _ = PostgresOrganizationRepository::create_invitation_audited;
    let _ = PostgresOrganizationRepository::accept_invitation_audited;

    // Host composition must install/apply the audit-log module; this test does so manually.
    let Some(db) = migrated_database().await else {
        return;
    };
    apply_migrations(&db.pool, AUDIT_LOG_MIGRATIONS)
        .await
        .expect("audit migrations apply");

    seed_user(&db.pool, "usr_audit_owner").await;
    seed_user(&db.pool, "usr_audit_member").await;

    let app = test_app(&db);
    let created = request_json(
        app.clone(),
        "POST",
        "/v1/organizations",
        Some("Bearer dev-user:usr_audit_owner"),
        Some(json!({ "name": "Audit Route Org", "slug": "audit-route-org" })),
    )
    .await;
    assert_eq!(created.0, StatusCode::OK);
    let organization_id = created.1["id"]
        .as_str()
        .expect("organization id")
        .to_owned();

    let repo = PostgresOrganizationRepository::new(db.pool.clone());
    let member_role = repo
        .member_role_for_organization(&organization_id)
        .await
        .expect("member role");
    let invited = request_json(
        app.clone(),
        "POST",
        &format!("/v1/organizations/{organization_id}/invitations"),
        Some("Bearer dev-user:usr_audit_owner"),
        Some(json!({
            "email": "audit-route-member@example.com",
            "role_id": member_role.id,
            "expires_at": (Utc::now() + Duration::days(1)).to_rfc3339(),
        })),
    )
    .await;
    assert_eq!(invited.0, StatusCode::OK);
    let token = invited.1["token"]
        .as_str()
        .expect("invite token")
        .to_owned();

    let accepted = request_json(
        app,
        "POST",
        &format!("/v1/organization-invitations/{token}/accept"),
        Some("Bearer dev-user:usr_audit_member"),
        None,
    )
    .await;
    assert_eq!(accepted.0, StatusCode::OK);

    let events = PostgresAuditLogRepository::new(db.pool.clone())
        .list_events(AuditEventFilter {
            module_name: Some(organization::module::MODULE_NAME.to_owned()),
            scope_type: Some("organization".to_owned()),
            scope_id: Some(organization_id),
            limit: 10,
            ..AuditEventFilter::default()
        })
        .await
        .expect("audit events");
    let event_names = events
        .iter()
        .map(|event| event.event_name.as_str())
        .collect::<Vec<_>>();

    assert!(event_names.contains(&"organization.created"));
    assert!(event_names.contains(&"organization.invitation_created"));
    assert!(event_names.contains(&"organization.invitation_accepted"));
    assert!(events.iter().all(|event| {
        event
            .correlation_id
            .as_deref()
            .is_some_and(|id| !id.is_empty())
    }));

    db.cleanup().await;
}

#[tokio::test]
async fn console_admin_actions_create_archive_and_manage_roles() {
    let Some(db) = migrated_database().await else {
        return;
    };

    seed_user(&db.pool, "usr_console_owner").await;

    let repo = Arc::new(PostgresOrganizationRepository::new(db.pool.clone()));
    let admin = OrganizationAdminData::new(repo.clone());
    let created = admin
        .invoke(
            "create_organization",
            json!({
                "name": "Console Org",
                "slug": "console-org",
                "owner_auth_user_id": "usr_console_owner"
            }),
        )
        .await
        .expect("create organization action");

    let organization_id = created["id"].as_str().expect("organization id");
    assert_eq!(created["name"], "Console Org");
    assert_eq!(created["slug"], "console-org");

    let custom_role = admin
        .invoke(
            "create_role",
            json!({
                "organization_id": organization_id,
                "name": "billing",
                "permissions": [
                    "organization.read",
                    "billing.invoices.read",
                    "billing.invoices.manage"
                ]
            }),
        )
        .await
        .expect("create role action");
    let role_id = custom_role["id"].as_str().expect("role id");
    assert_eq!(custom_role["system_key"], Value::Null);
    assert_eq!(custom_role["permissions"][1], "billing.invoices.read");

    let updated = admin
        .invoke(
            "update_role_permissions",
            json!({
                "role_id": role_id,
                "permissions": ["organization.read", "billing.invoices.read"]
            }),
        )
        .await
        .expect("update role permissions action");
    assert_eq!(updated["updated"], true);

    let role_record = admin
        .get("roles", role_id)
        .await
        .expect("get role")
        .expect("role exists");
    assert_eq!(
        role_record["permissions"],
        json!(["organization.read", "billing.invoices.read"])
    );

    let owner_role = repo
        .owner_role_for_organization(organization_id)
        .await
        .expect("owner role");
    let rejected = admin
        .invoke(
            "update_role_permissions",
            json!({
                "role_id": owner_role.id,
                "permissions": ["organization.read"]
            }),
        )
        .await
        .expect_err("owner role permissions are protected");
    assert!(
        rejected
            .to_string()
            .contains("owner role permissions cannot be changed")
    );

    let archived = admin
        .invoke(
            "archive_organization",
            json!({ "organization_id": organization_id }),
        )
        .await
        .expect("archive organization action");
    assert_eq!(archived["archived"], true);

    let organization_record = admin
        .get("organizations", organization_id)
        .await
        .expect("get organization")
        .expect("organization exists");
    assert!(organization_record["archived_at"].as_str().is_some());

    let visible_to_owner = organization::public::list_user_organizations(
        &db.pool,
        &AuthUserId("usr_console_owner".to_owned()),
    )
    .await
    .expect("list user organizations");
    assert!(visible_to_owner.is_empty());

    db.cleanup().await;
}

#[tokio::test]
async fn archived_organizations_reject_invitations_memberships_and_role_updates() {
    let Some(db) = migrated_database().await else {
        return;
    };

    seed_user(&db.pool, "usr_archive_owner").await;
    seed_user(&db.pool, "usr_archive_invited").await;
    seed_user(&db.pool, "usr_archive_member").await;

    let now = Utc::now();
    let owner = AuthUserId("usr_archive_owner".to_owned());
    let repo = Arc::new(PostgresOrganizationRepository::new(db.pool.clone()));
    let organization = repo
        .create_organization_with_owner("Archive Org", "archive-org", &owner, now)
        .await
        .expect("organization created");
    let member_role = repo
        .member_role_for_organization(&organization.id)
        .await
        .expect("member role");
    let invitation = repo
        .create_invitation(
            &organization.id,
            "before-archive@example.com",
            &member_role.id,
            now + Duration::days(1),
            now,
        )
        .await
        .expect("pre-archive invitation");
    let membership_invitation = repo
        .create_invitation(
            &organization.id,
            "member-before-archive@example.com",
            &member_role.id,
            now + Duration::days(1),
            now,
        )
        .await
        .expect("pre-archive membership invitation");
    let membership = repo
        .accept_invitation(
            &membership_invitation.token,
            &AuthUserId("usr_archive_member".to_owned()),
            now,
        )
        .await
        .expect("pre-archive membership accepted");
    let custom_role = repo
        .create_role(
            &organization.id,
            " billing ",
            &[
                " organization.read ".to_owned(),
                "billing.invoices.read".to_owned(),
                "organization.read".to_owned(),
            ],
            now,
        )
        .await
        .expect("custom role");
    assert_eq!(
        custom_role.permissions,
        vec!["organization.read", "billing.invoices.read"]
    );

    assert!(
        repo.archive_organization(&organization.id, now)
            .await
            .expect("archive organization")
    );

    let rejected_invitation = repo
        .create_invitation(
            &organization.id,
            "after-archive@example.com",
            &member_role.id,
            now + Duration::days(1),
            now,
        )
        .await
        .expect_err("archived organization rejects new invitations");
    assert!(
        rejected_invitation
            .to_string()
            .contains("organization is archived")
    );

    let rejected_acceptance = repo
        .accept_invitation(
            &invitation.token,
            &AuthUserId("usr_archive_invited".to_owned()),
            now,
        )
        .await
        .expect_err("archived organization rejects outstanding invitations");
    assert!(
        rejected_acceptance
            .to_string()
            .contains("organization is archived")
    );

    let rejected_role_update = repo
        .update_role_permissions(&custom_role.id, &["organization.read".to_owned()], now)
        .await
        .expect_err("archived organization rejects role permission updates");
    assert!(
        rejected_role_update
            .to_string()
            .contains("organization is archived")
    );

    let rejected_member_update = repo
        .update_member_role(&membership.id, &custom_role.id, now)
        .await
        .expect_err("archived organization rejects member role updates");
    assert!(
        rejected_member_update
            .to_string()
            .contains("organization is archived")
    );

    let rejected_member_removal = repo
        .remove_member(&membership.id, now)
        .await
        .expect_err("archived organization rejects member removal");
    assert!(
        rejected_member_removal
            .to_string()
            .contains("organization is archived")
    );

    db.cleanup().await;
}

async fn migrated_database() -> Option<TestDatabase> {
    let db = TestDatabase::create().await?;
    let migrations = PLATFORM_MIGRATIONS
        .iter()
        .chain(RUNTIME_MIGRATIONS)
        .chain(auth::migrations::AUTH_MIGRATIONS)
        .chain(organization::migrations::ORGANIZATION_MIGRATIONS)
        .copied()
        .collect::<Vec<_>>();
    apply_migrations(&db.pool, &migrations)
        .await
        .expect("migrations apply");
    Some(db)
}

async fn seed_user(pool: &platform_core::DbPool, id: &str) {
    PostgresAuthUserRepository::new(pool.clone())
        .insert(&AuthUser {
            id: AuthUserId(id.to_owned()),
            created_at: Utc::now(),
            disabled_at: None,
            disabled_reason: None,
            disabled_until: None,
        })
        .await
        .expect("insert auth user");
}

fn test_app(db: &TestDatabase) -> axum::Router {
    let (router, _) = organization::routes::router().split_for_parts();
    let ctx = platform_core::AppContext::new(
        test_config(db.url.clone()),
        db.pool.clone(),
        Arc::new(LoggingEventPublisher),
    );
    router
        .layer(middleware::from_fn_with_state(
            ctx.clone(),
            request_context_middleware,
        ))
        .with_state(ctx)
}

fn test_config(database_url: String) -> AppConfig {
    let mut config = AppConfig::from_env();
    config.database.url = database_url;
    config.database.max_connections = 1;
    config.service.environment = "local".to_owned();
    config.service.name = "organization-test".to_owned();
    config.module_sources = platform_core::ModuleSourcesConfig::default();
    config.modules.clear();
    config
}

async fn request_json(
    app: axum::Router,
    method: &str,
    uri: &str,
    authorization: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(authorization) = authorization {
        builder = builder.header("authorization", authorization);
    }
    let request_body = if let Some(value) = body {
        builder = builder.header("content-type", "application/json");
        Body::from(serde_json::to_vec(&value).expect("serialize body"))
    } else {
        Body::empty()
    };
    let response = app
        .oneshot(builder.body(request_body).expect("request"))
        .await
        .expect("response");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json body")
    };
    (status, value)
}
