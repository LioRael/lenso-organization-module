use crate::models::{Membership, Organization};
use crate::repositories::PostgresOrganizationRepository;
use auth::public::AuthUserId;
use chrono::{DateTime, Utc};
use platform_core::{AppResult, DbPool};

pub async fn create_organization_with_owner(
    pool: &DbPool,
    name: &str,
    slug: &str,
    owner_auth_user_id: &AuthUserId,
    now: DateTime<Utc>,
) -> AppResult<Organization> {
    PostgresOrganizationRepository::new(pool.clone())
        .create_organization_with_owner(name, slug, owner_auth_user_id, now)
        .await
}

pub async fn has_permission(
    pool: &DbPool,
    organization_id: &str,
    auth_user_id: &AuthUserId,
    permission: &str,
) -> AppResult<bool> {
    PostgresOrganizationRepository::new(pool.clone())
        .has_permission(organization_id, auth_user_id, permission)
        .await
}

pub async fn list_user_organizations(
    pool: &DbPool,
    auth_user_id: &AuthUserId,
) -> AppResult<Vec<Organization>> {
    PostgresOrganizationRepository::new(pool.clone())
        .list_user_organizations(auth_user_id)
        .await
}

pub async fn accept_invitation(
    pool: &DbPool,
    token: &str,
    auth_user_id: &AuthUserId,
    now: DateTime<Utc>,
) -> AppResult<Membership> {
    PostgresOrganizationRepository::new(pool.clone())
        .accept_invitation(token, auth_user_id, now)
        .await
}
