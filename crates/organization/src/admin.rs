use crate::repositories::PostgresOrganizationRepository;
use chrono::{DateTime, Utc};
use platform_core::{AppError, AppResult, ErrorCode};
use platform_module::{AdminActionSource, AdminDataSource, AdminListQuery, AdminPage};
use serde_json::Value;
use std::sync::Arc;

pub const CREATE_INVITATION_ACTION: &str = "create_invitation";
pub const REVOKE_INVITATION_ACTION: &str = "revoke_invitation";
pub const UPDATE_MEMBER_ROLE_ACTION: &str = "update_member_role";
pub const REMOVE_MEMBER_ACTION: &str = "remove_member";

#[derive(Debug)]
pub struct OrganizationAdminData {
    repository: Arc<PostgresOrganizationRepository>,
}

impl OrganizationAdminData {
    #[must_use]
    pub fn new(repository: Arc<PostgresOrganizationRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait::async_trait]
impl AdminDataSource for OrganizationAdminData {
    async fn list(&self, entity: &str, query: &AdminListQuery) -> AppResult<AdminPage> {
        let rows = self
            .repository
            .list(
                entity,
                query.limit.saturating_add(1),
                query.cursor.as_deref(),
            )
            .await?;
        let has_more = rows.len() as i64 > query.limit.max(0);
        let take = rows.len().min(query.limit.max(0) as usize);
        let records = rows.into_iter().take(take).collect::<Vec<_>>();
        let next_cursor = has_more
            .then(|| {
                records
                    .last()
                    .and_then(|row| row.get("id"))
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .flatten();
        Ok(AdminPage {
            records,
            next_cursor,
        })
    }

    async fn get(&self, entity: &str, id: &str) -> AppResult<Option<Value>> {
        self.repository.get(entity, id).await
    }
}

#[async_trait::async_trait]
impl AdminActionSource for OrganizationAdminData {
    async fn invoke(&self, action: &str, input: Value) -> AppResult<Value> {
        match action {
            CREATE_INVITATION_ACTION => {
                let organization_id = required_string(&input, "organization_id")?;
                let email = required_string(&input, "email")?;
                let role_id = required_string(&input, "role_id")?;
                let expires_at = required_timestamp(&input, "expires_at")?;
                let created = self
                    .repository
                    .create_invitation(organization_id, email, role_id, expires_at, Utc::now())
                    .await?;
                Ok(serde_json::json!({
                    "id": created.invitation.id,
                    "organization_id": created.invitation.organization_id,
                    "email": created.invitation.email,
                    "role_id": created.invitation.role_id,
                    "token": created.token,
                    "expires_at": created.invitation.expires_at,
                }))
            }
            REVOKE_INVITATION_ACTION => {
                let invitation_id = required_string(&input, "invitation_id")?;
                let revoked = self
                    .repository
                    .revoke_invitation(invitation_id, Utc::now())
                    .await?;
                Ok(serde_json::json!({ "invitation_id": invitation_id, "revoked": revoked }))
            }
            UPDATE_MEMBER_ROLE_ACTION => {
                let membership_id = required_string(&input, "membership_id")?;
                let role_id = required_string(&input, "role_id")?;
                let updated = self
                    .repository
                    .update_member_role(membership_id, role_id, Utc::now())
                    .await?;
                Ok(
                    serde_json::json!({ "membership_id": membership_id, "role_id": role_id, "updated": updated }),
                )
            }
            REMOVE_MEMBER_ACTION => {
                let membership_id = required_string(&input, "membership_id")?;
                let removed = self
                    .repository
                    .remove_member(membership_id, Utc::now())
                    .await?;
                Ok(serde_json::json!({ "membership_id": membership_id, "removed": removed }))
            }
            other => Err(AppError::new(
                ErrorCode::NotFound,
                format!("unknown organization admin action: {other}"),
            )),
        }
    }
}

fn required_string<'a>(input: &'a Value, name: &str) -> AppResult<&'a str> {
    input
        .get(name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::new(ErrorCode::Validation, format!("{name} is required")))
}

fn required_timestamp(input: &Value, name: &str) -> AppResult<DateTime<Utc>> {
    let value = required_string(input, name)?;
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| AppError::new(ErrorCode::Validation, format!("{name} must be RFC3339")))
}
