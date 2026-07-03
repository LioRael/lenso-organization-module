use crate::models::{CreatedInvitation, Invitation, Membership, Organization, Role};
use auth::public::AuthUserId;
use chrono::{DateTime, Utc};
use platform_core::{AppError, AppResult, DbPool, ErrorCode};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;

pub const OWNER_PERMISSIONS: &[&str] = &[
    "organization.read",
    "organization.manage",
    "organization.members.manage",
    "organization.roles.manage",
    "organization.invitations.manage",
];
pub const ADMIN_PERMISSIONS: &[&str] = &[
    "organization.read",
    "organization.members.manage",
    "organization.invitations.manage",
];
pub const MEMBER_PERMISSIONS: &[&str] = &["organization.read"];

#[derive(Debug, Clone)]
pub struct PostgresOrganizationRepository {
    pool: DbPool,
}

impl PostgresOrganizationRepository {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create_organization_with_owner(
        &self,
        name: &str,
        slug: &str,
        owner_auth_user_id: &AuthUserId,
        now: DateTime<Utc>,
    ) -> AppResult<Organization> {
        let name = required_trimmed(name, "name")?;
        let slug = required_trimmed(slug, "slug")?;
        let organization_id = new_id("org");
        let owner_role_id = new_id("org_role");
        let admin_role_id = new_id("org_role");
        let member_role_id = new_id("org_role");
        let membership_id = new_id("org_member");
        let mut tx = self.pool.begin().await.map_err(map_sql_error)?;

        let organization = sqlx::query_as::<_, OrganizationRow>(
            r#"
            insert into organization.organizations (id, name, slug, created_at, updated_at, archived_at)
            values ($1, $2, $3, $4, $4, null)
            returning id, name, slug, created_at, updated_at, archived_at
            "#,
        )
        .bind(&organization_id)
        .bind(name)
        .bind(slug)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map(organization_from_row)
        .map_err(map_sql_error)?;

        insert_role(
            &mut tx,
            &owner_role_id,
            &organization_id,
            "owner",
            OWNER_PERMISSIONS,
            Some("owner"),
            now,
        )
        .await?;
        insert_role(
            &mut tx,
            &admin_role_id,
            &organization_id,
            "admin",
            ADMIN_PERMISSIONS,
            Some("admin"),
            now,
        )
        .await?;
        insert_role(
            &mut tx,
            &member_role_id,
            &organization_id,
            "member",
            MEMBER_PERMISSIONS,
            Some("member"),
            now,
        )
        .await?;

        sqlx::query(
            r#"
            insert into organization.memberships (id, organization_id, auth_user_id, role_id, created_at, updated_at, removed_at)
            values ($1, $2, $3, $4, $5, $5, null)
            "#,
        )
        .bind(&membership_id)
        .bind(&organization_id)
        .bind(&owner_auth_user_id.0)
        .bind(&owner_role_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_sql_error)?;

        tx.commit().await.map_err(map_sql_error)?;
        Ok(organization)
    }

    pub async fn list_user_organizations(
        &self,
        auth_user_id: &AuthUserId,
    ) -> AppResult<Vec<Organization>> {
        sqlx::query_as::<_, OrganizationRow>(
            r#"
            select organizations.id, organizations.name, organizations.slug,
                   organizations.created_at, organizations.updated_at, organizations.archived_at
            from organization.organizations organizations
            join organization.memberships memberships
              on memberships.organization_id = organizations.id
             and memberships.removed_at is null
            where memberships.auth_user_id = $1
              and organizations.archived_at is null
            order by organizations.id asc
            "#,
        )
        .bind(&auth_user_id.0)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(organization_from_row).collect())
        .map_err(map_sql_error)
    }

    pub async fn has_permission(
        &self,
        organization_id: &str,
        auth_user_id: &AuthUserId,
        permission: &str,
    ) -> AppResult<bool> {
        sqlx::query_scalar::<_, bool>(
            r#"
            select exists(
                select 1
                from organization.memberships memberships
                join organization.roles roles on roles.id = memberships.role_id
                join organization.organizations organizations on organizations.id = memberships.organization_id
                where memberships.organization_id = $1
                  and memberships.auth_user_id = $2
                  and memberships.removed_at is null
                  and organizations.archived_at is null
                  and roles.permissions ? $3
            )
            "#,
        )
        .bind(organization_id)
        .bind(&auth_user_id.0)
        .bind(permission)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sql_error)
    }

    pub async fn list(
        &self,
        entity: &str,
        limit: i64,
        cursor: Option<&str>,
    ) -> AppResult<Vec<Value>> {
        match entity {
            "organizations" => self
                .list_organizations(limit, cursor)
                .await
                .map(|rows| rows.into_iter().map(organization_to_value).collect()),
            "roles" => self
                .list_roles(limit, cursor)
                .await
                .map(|rows| rows.into_iter().map(role_to_value).collect()),
            "memberships" => self
                .list_memberships(limit, cursor)
                .await
                .map(|rows| rows.into_iter().map(membership_to_value).collect()),
            "invitations" => self
                .list_invitations(limit, cursor)
                .await
                .map(|rows| rows.into_iter().map(invitation_to_value).collect()),
            other => Err(unknown_entity(other)),
        }
    }

    pub async fn get(&self, entity: &str, id: &str) -> AppResult<Option<Value>> {
        match entity {
            "organizations" => self
                .find_organization(id)
                .await
                .map(|row| row.map(organization_to_value)),
            "roles" => self.find_role(id).await.map(|row| row.map(role_to_value)),
            "memberships" => self
                .find_membership(id)
                .await
                .map(|row| row.map(membership_to_value)),
            "invitations" => self
                .find_invitation(id)
                .await
                .map(|row| row.map(invitation_to_value)),
            other => Err(unknown_entity(other)),
        }
    }

    pub async fn list_members(&self, organization_id: &str) -> AppResult<Vec<Membership>> {
        sqlx::query_as::<_, MembershipRow>(
            r#"
            select memberships.id, memberships.organization_id, memberships.auth_user_id,
                   memberships.role_id, roles.name, memberships.created_at,
                   memberships.updated_at, memberships.removed_at
            from organization.memberships memberships
            join organization.roles roles on roles.id = memberships.role_id
            where memberships.organization_id = $1
              and memberships.removed_at is null
            order by memberships.id asc
            "#,
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(membership_from_row).collect())
        .map_err(map_sql_error)
    }

    pub async fn create_invitation(
        &self,
        organization_id: &str,
        email: &str,
        role_id: &str,
        expires_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> AppResult<CreatedInvitation> {
        let email = required_trimmed(email, "email")?;
        if expires_at <= now {
            return Err(AppError::new(
                ErrorCode::Validation,
                "expires_at must be in the future",
            ));
        }
        let role = self
            .find_role(role_id)
            .await?
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, "role not found"))?;
        if role.organization_id != organization_id {
            return Err(AppError::new(
                ErrorCode::Validation,
                "role does not belong to organization",
            ));
        }
        let invitation_id = new_id("org_invite");
        let token = new_id("org_inv_token");
        let token_hash = token_hash(&token);
        let invitation = sqlx::query_as::<_, InvitationRow>(
            r#"
            insert into organization.invitations (
                id, organization_id, email, role_id, token_hash, expires_at,
                created_at, updated_at, accepted_at, revoked_at
            )
            values ($1, $2, $3, $4, $5, $6, $7, $7, null, null)
            returning id, organization_id, email, role_id, expires_at, created_at, updated_at, accepted_at, revoked_at
            "#,
        )
        .bind(&invitation_id)
        .bind(organization_id)
        .bind(email)
        .bind(role_id)
        .bind(token_hash)
        .bind(expires_at)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map(invitation_from_row)
        .map_err(map_sql_error)?;

        Ok(CreatedInvitation { invitation, token })
    }

    pub async fn accept_invitation(
        &self,
        token: &str,
        auth_user_id: &AuthUserId,
        now: DateTime<Utc>,
    ) -> AppResult<Membership> {
        let token = required_trimmed(token, "token")?;
        let token_hash = token_hash(token);
        let mut tx = self.pool.begin().await.map_err(map_sql_error)?;
        let invitation = sqlx::query_as::<_, InvitationRow>(
            r#"
            select id, organization_id, email, role_id, expires_at, created_at, updated_at, accepted_at, revoked_at
            from organization.invitations
            where token_hash = $1
              and accepted_at is null
              and revoked_at is null
            for update
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sql_error)?
        .map(invitation_from_row)
        .ok_or_else(|| AppError::new(ErrorCode::NotFound, "invitation not found"))?;

        if invitation.expires_at <= now {
            return Err(AppError::new(
                ErrorCode::Validation,
                "invitation has expired",
            ));
        }

        let membership_id = new_id("org_member");
        let membership = sqlx::query_as::<_, MembershipRow>(
            r#"
            insert into organization.memberships (
                id, organization_id, auth_user_id, role_id, created_at, updated_at, removed_at
            )
            values ($1, $2, $3, $4, $5, $5, null)
            returning id, organization_id, auth_user_id, role_id, (
                select name from organization.roles where id = $4
            ), created_at, updated_at, removed_at
            "#,
        )
        .bind(&membership_id)
        .bind(&invitation.organization_id)
        .bind(&auth_user_id.0)
        .bind(&invitation.role_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map(membership_from_row)
        .map_err(map_sql_error)?;

        sqlx::query(
            "update organization.invitations set accepted_at = $2, updated_at = $2 where id = $1",
        )
        .bind(&invitation.id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_sql_error)?;

        tx.commit().await.map_err(map_sql_error)?;
        Ok(membership)
    }

    pub async fn revoke_invitation(
        &self,
        invitation_id: &str,
        now: DateTime<Utc>,
    ) -> AppResult<bool> {
        sqlx::query_scalar::<_, String>(
            r#"
            update organization.invitations
            set revoked_at = $2, updated_at = $2
            where id = $1
              and accepted_at is null
              and revoked_at is null
            returning id
            "#,
        )
        .bind(invitation_id)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.is_some())
        .map_err(map_sql_error)
    }

    pub async fn update_member_role(
        &self,
        membership_id: &str,
        role_id: &str,
        now: DateTime<Utc>,
    ) -> AppResult<bool> {
        let membership = self
            .find_membership(membership_id)
            .await?
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, "membership not found"))?;
        let role = self
            .find_role(role_id)
            .await?
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, "role not found"))?;
        if role.organization_id != membership.organization_id {
            return Err(AppError::new(
                ErrorCode::Validation,
                "role does not belong to membership organization",
            ));
        }
        sqlx::query_scalar::<_, String>(
            r#"
            update organization.memberships
            set role_id = $2, updated_at = $3
            where id = $1 and removed_at is null
            returning id
            "#,
        )
        .bind(membership_id)
        .bind(role_id)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.is_some())
        .map_err(map_sql_error)
    }

    pub async fn remove_member(&self, membership_id: &str, now: DateTime<Utc>) -> AppResult<bool> {
        sqlx::query_scalar::<_, String>(
            r#"
            update organization.memberships
            set removed_at = $2, updated_at = $2
            where id = $1 and removed_at is null
            returning id
            "#,
        )
        .bind(membership_id)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.is_some())
        .map_err(map_sql_error)
    }

    pub async fn owner_role_for_organization(&self, organization_id: &str) -> AppResult<Role> {
        self.role_by_system_key(organization_id, "owner").await
    }

    pub async fn member_role_for_organization(&self, organization_id: &str) -> AppResult<Role> {
        self.role_by_system_key(organization_id, "member").await
    }

    async fn role_by_system_key(&self, organization_id: &str, system_key: &str) -> AppResult<Role> {
        sqlx::query_as::<_, RoleRow>(
            r#"
            select id, organization_id, name, permissions, system_key, created_at, updated_at
            from organization.roles
            where organization_id = $1 and system_key = $2
            "#,
        )
        .bind(organization_id)
        .bind(system_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sql_error)?
        .map(role_from_row)
        .ok_or_else(|| AppError::new(ErrorCode::NotFound, "role not found"))
    }

    async fn find_organization(&self, id: &str) -> AppResult<Option<Organization>> {
        sqlx::query_as::<_, OrganizationRow>(
            "select id, name, slug, created_at, updated_at, archived_at from organization.organizations where id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(organization_from_row))
        .map_err(map_sql_error)
    }

    async fn find_role(&self, id: &str) -> AppResult<Option<Role>> {
        sqlx::query_as::<_, RoleRow>(
            "select id, organization_id, name, permissions, system_key, created_at, updated_at from organization.roles where id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(role_from_row))
        .map_err(map_sql_error)
    }

    async fn find_membership(&self, id: &str) -> AppResult<Option<Membership>> {
        sqlx::query_as::<_, MembershipRow>(
            r#"
            select memberships.id, memberships.organization_id, memberships.auth_user_id,
                   memberships.role_id, roles.name, memberships.created_at,
                   memberships.updated_at, memberships.removed_at
            from organization.memberships memberships
            join organization.roles roles on roles.id = memberships.role_id
            where memberships.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(membership_from_row))
        .map_err(map_sql_error)
    }

    async fn find_invitation(&self, id: &str) -> AppResult<Option<Invitation>> {
        sqlx::query_as::<_, InvitationRow>(
            "select id, organization_id, email, role_id, expires_at, created_at, updated_at, accepted_at, revoked_at from organization.invitations where id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(invitation_from_row))
        .map_err(map_sql_error)
    }

    async fn list_organizations(
        &self,
        limit: i64,
        cursor: Option<&str>,
    ) -> AppResult<Vec<Organization>> {
        let sql = match cursor {
            Some(_) => {
                "select id, name, slug, created_at, updated_at, archived_at from organization.organizations where id > $1 order by id asc limit $2"
            }
            None => {
                "select id, name, slug, created_at, updated_at, archived_at from organization.organizations order by id asc limit $1"
            }
        };
        let query = sqlx::query_as::<_, OrganizationRow>(sql);
        let rows = if let Some(cursor) = cursor {
            query.bind(cursor).bind(limit).fetch_all(&self.pool).await
        } else {
            query.bind(limit).fetch_all(&self.pool).await
        }
        .map_err(map_sql_error)?;
        Ok(rows.into_iter().map(organization_from_row).collect())
    }

    async fn list_roles(&self, limit: i64, cursor: Option<&str>) -> AppResult<Vec<Role>> {
        let sql = match cursor {
            Some(_) => {
                "select id, organization_id, name, permissions, system_key, created_at, updated_at from organization.roles where id > $1 order by id asc limit $2"
            }
            None => {
                "select id, organization_id, name, permissions, system_key, created_at, updated_at from organization.roles order by id asc limit $1"
            }
        };
        let query = sqlx::query_as::<_, RoleRow>(sql);
        let rows = if let Some(cursor) = cursor {
            query.bind(cursor).bind(limit).fetch_all(&self.pool).await
        } else {
            query.bind(limit).fetch_all(&self.pool).await
        }
        .map_err(map_sql_error)?;
        Ok(rows.into_iter().map(role_from_row).collect())
    }

    async fn list_memberships(
        &self,
        limit: i64,
        cursor: Option<&str>,
    ) -> AppResult<Vec<Membership>> {
        let sql = match cursor {
            Some(_) => {
                "select memberships.id, memberships.organization_id, memberships.auth_user_id, memberships.role_id, roles.name, memberships.created_at, memberships.updated_at, memberships.removed_at from organization.memberships memberships join organization.roles roles on roles.id = memberships.role_id where memberships.id > $1 order by memberships.id asc limit $2"
            }
            None => {
                "select memberships.id, memberships.organization_id, memberships.auth_user_id, memberships.role_id, roles.name, memberships.created_at, memberships.updated_at, memberships.removed_at from organization.memberships memberships join organization.roles roles on roles.id = memberships.role_id order by memberships.id asc limit $1"
            }
        };
        let query = sqlx::query_as::<_, MembershipRow>(sql);
        let rows = if let Some(cursor) = cursor {
            query.bind(cursor).bind(limit).fetch_all(&self.pool).await
        } else {
            query.bind(limit).fetch_all(&self.pool).await
        }
        .map_err(map_sql_error)?;
        Ok(rows.into_iter().map(membership_from_row).collect())
    }

    async fn list_invitations(
        &self,
        limit: i64,
        cursor: Option<&str>,
    ) -> AppResult<Vec<Invitation>> {
        let sql = match cursor {
            Some(_) => {
                "select id, organization_id, email, role_id, expires_at, created_at, updated_at, accepted_at, revoked_at from organization.invitations where id > $1 order by id asc limit $2"
            }
            None => {
                "select id, organization_id, email, role_id, expires_at, created_at, updated_at, accepted_at, revoked_at from organization.invitations order by id asc limit $1"
            }
        };
        let query = sqlx::query_as::<_, InvitationRow>(sql);
        let rows = if let Some(cursor) = cursor {
            query.bind(cursor).bind(limit).fetch_all(&self.pool).await
        } else {
            query.bind(limit).fetch_all(&self.pool).await
        }
        .map_err(map_sql_error)?;
        Ok(rows.into_iter().map(invitation_from_row).collect())
    }
}

async fn insert_role(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: &str,
    organization_id: &str,
    name: &str,
    permissions: &[&str],
    system_key: Option<&str>,
    now: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        insert into organization.roles (id, organization_id, name, permissions, system_key, created_at, updated_at)
        values ($1, $2, $3, $4, $5, $6, $6)
        "#,
    )
    .bind(id)
    .bind(organization_id)
    .bind(name)
    .bind(serde_json::to_value(permissions).expect("permissions serializes"))
    .bind(system_key)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map(|_| ())
    .map_err(map_sql_error)
}

type OrganizationRow = (
    String,
    String,
    String,
    DateTime<Utc>,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
);
type RoleRow = (
    String,
    String,
    String,
    Value,
    Option<String>,
    DateTime<Utc>,
    DateTime<Utc>,
);
type MembershipRow = (
    String,
    String,
    String,
    String,
    Option<String>,
    DateTime<Utc>,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
);
type InvitationRow = (
    String,
    String,
    String,
    String,
    DateTime<Utc>,
    DateTime<Utc>,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
);

fn organization_from_row(row: OrganizationRow) -> Organization {
    let (id, name, slug, created_at, updated_at, archived_at) = row;
    Organization {
        id,
        name,
        slug,
        created_at,
        updated_at,
        archived_at,
    }
}

fn role_from_row(row: RoleRow) -> Role {
    let (id, organization_id, name, permissions, system_key, created_at, updated_at) = row;
    let permissions = permissions
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect();
    Role {
        id,
        organization_id,
        name,
        permissions,
        system_key,
        created_at,
        updated_at,
    }
}

fn membership_from_row(row: MembershipRow) -> Membership {
    let (id, organization_id, auth_user_id, role_id, role_name, created_at, updated_at, removed_at) =
        row;
    Membership {
        id,
        organization_id,
        auth_user_id: AuthUserId(auth_user_id),
        role_id,
        role_name,
        created_at,
        updated_at,
        removed_at,
    }
}

fn invitation_from_row(row: InvitationRow) -> Invitation {
    let (
        id,
        organization_id,
        email,
        role_id,
        expires_at,
        created_at,
        updated_at,
        accepted_at,
        revoked_at,
    ) = row;
    Invitation {
        id,
        organization_id,
        email,
        role_id,
        expires_at,
        created_at,
        updated_at,
        accepted_at,
        revoked_at,
    }
}

pub fn organization_to_value(organization: Organization) -> Value {
    serde_json::json!({
        "id": organization.id,
        "name": organization.name,
        "slug": organization.slug,
        "created_at": organization.created_at,
        "updated_at": organization.updated_at,
        "archived_at": organization.archived_at,
    })
}

pub fn role_to_value(role: Role) -> Value {
    serde_json::json!({
        "id": role.id,
        "organization_id": role.organization_id,
        "name": role.name,
        "permissions": role.permissions,
        "system_key": role.system_key,
        "created_at": role.created_at,
        "updated_at": role.updated_at,
    })
}

pub fn membership_to_value(membership: Membership) -> Value {
    serde_json::json!({
        "id": membership.id,
        "organization_id": membership.organization_id,
        "auth_user_id": membership.auth_user_id.0,
        "role_id": membership.role_id,
        "role_name": membership.role_name,
        "created_at": membership.created_at,
        "updated_at": membership.updated_at,
        "removed_at": membership.removed_at,
    })
}

pub fn invitation_to_value(invitation: Invitation) -> Value {
    serde_json::json!({
        "id": invitation.id,
        "organization_id": invitation.organization_id,
        "email": invitation.email,
        "role_id": invitation.role_id,
        "expires_at": invitation.expires_at,
        "created_at": invitation.created_at,
        "updated_at": invitation.updated_at,
        "accepted_at": invitation.accepted_at,
        "revoked_at": invitation.revoked_at,
    })
}

pub fn token_hash(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    let mut hash = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hash, "{byte:02x}");
    }
    hash
}

fn new_id(prefix: &str) -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("OS randomness should be available");
    let mut id = String::with_capacity(prefix.len() + 1 + bytes.len() * 2);
    id.push_str(prefix);
    id.push('_');
    for byte in bytes {
        let _ = write!(id, "{byte:02x}");
    }
    id
}

fn required_trimmed<'a>(value: &'a str, name: &str) -> AppResult<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::new(
            ErrorCode::Validation,
            format!("{name} is required"),
        ));
    }
    Ok(value)
}

fn unknown_entity(entity: &str) -> AppError {
    AppError::new(
        ErrorCode::NotFound,
        format!("unknown admin entity: {entity}"),
    )
}

fn map_sql_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database) = &error {
        if database.is_unique_violation() {
            return AppError::new(ErrorCode::Conflict, database.message().to_owned());
        }
        if database.is_foreign_key_violation() {
            return AppError::new(ErrorCode::Validation, database.message().to_owned());
        }
    }
    AppError::new(ErrorCode::Internal, error.to_string())
}
