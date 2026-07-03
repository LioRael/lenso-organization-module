use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrganizationResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OrganizationListResponse {
    pub organizations: Vec<OrganizationResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemberResponse {
    pub id: String,
    pub organization_id: String,
    pub auth_user_id: String,
    pub role_id: String,
    pub role_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub removed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemberListResponse {
    pub members: Vec<MemberResponse>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateInvitationRequest {
    pub email: String,
    pub role_id: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateInvitationResponse {
    pub id: String,
    pub organization_id: String,
    pub email: String,
    pub role_id: String,
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AcceptInvitationResponse {
    pub membership: MemberResponse,
}
