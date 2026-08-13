use crate::models::{Membership, Organization};
use crate::repositories::PostgresOrganizationRepository;
use auth::public::AuthUserId;
use chrono::{DateTime, Utc};
use platform_core::{AppResult, DbPool};

#[cfg(feature = "notification")]
pub use crate::notification::{CreateInvitationDelivery, InvitationDeliveryReceipt};

#[cfg(feature = "notification")]
pub async fn create_invitation_delivery(
    pool: &DbPool,
    request_ctx: &platform_core::RequestContext,
    organization_id: &str,
    request: &CreateInvitationDelivery,
    idempotency_key: &str,
    public_invitation_base_url: &str,
    now: DateTime<Utc>,
) -> AppResult<InvitationDeliveryReceipt> {
    crate::notification::create_invitation_delivery(
        &PostgresOrganizationRepository::new(pool.clone()),
        request_ctx,
        organization_id,
        request,
        idempotency_key,
        public_invitation_base_url,
        now,
    )
    .await
}

/// Variant for host-owned secret providers and deterministic integration tests.
#[cfg(feature = "notification")]
pub async fn create_invitation_delivery_with_protector(
    pool: &DbPool,
    request_ctx: &platform_core::RequestContext,
    organization_id: &str,
    request: &CreateInvitationDelivery,
    idempotency_key: &str,
    public_invitation_base_url: &str,
    now: DateTime<Utc>,
    protector: &dyn notification::snapshot::SnapshotProtector,
) -> AppResult<InvitationDeliveryReceipt> {
    crate::notification::create_invitation_delivery_with_protector(
        &PostgresOrganizationRepository::new(pool.clone()),
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
