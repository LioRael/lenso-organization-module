use crate::dto::{
    AcceptInvitationResponse, CreateInvitationRequest, CreateInvitationResponse,
    CreateOrganizationRequest, MemberListResponse, MemberResponse, OrganizationListResponse,
    OrganizationResponse,
};
#[cfg(feature = "notification")]
use crate::dto::{CreateInvitationDeliveryRequest, CreateInvitationDeliveryResponse};
use crate::models::{Membership, Organization};
use crate::module::{ORGANIZATION_INVITATIONS_MANAGE, ORGANIZATION_READ};
use crate::repositories::PostgresOrganizationRepository;
use auth::public::AuthUserId;
use axum::Json;
use axum::extract::{Path, State};
#[cfg(feature = "notification")]
use axum::http::HeaderMap;
use platform_core::{AppContext, AppError, ErrorCode};
use platform_http::responses::json;
use platform_http::{
    ApiErrorResponse, ApiOpenApiRouter, ErrorResponse, HttpRequestContext, JsonBody, OpenApiRouter,
    UserActor, routes,
};

pub fn router() -> ApiOpenApiRouter {
    let router = OpenApiRouter::new()
        .routes(routes!(create_organization))
        .routes(routes!(list_organizations))
        .routes(routes!(list_members))
        .routes(routes!(create_invitation))
        .routes(routes!(accept_invitation));
    #[cfg(feature = "notification")]
    let router = router.routes(routes!(create_invitation_delivery));
    router
}

#[cfg(feature = "notification")]
#[utoipa::path(
    post,
    path = "/v1/organizations/{id}/invitation-deliveries",
    operation_id = "organization_create_invitation_delivery",
    tag = "organization",
    params(
        ("id" = String, Path, description = "Organization id"),
        ("Idempotency-Key" = String, Header, description = "Stable request identity")
    ),
    request_body(content = CreateInvitationDeliveryRequest, content_type = "application/json"),
    responses(
        (status = 200, description = "Invitation and delivery created atomically", body = CreateInvitationDeliveryResponse),
        (status = 400, description = "Invalid invitation delivery request", body = ErrorResponse),
        (status = 403, description = "Organization permission is required", body = ErrorResponse),
        (status = 409, description = "Idempotency conflict", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
async fn create_invitation_delivery(
    State(ctx): State<AppContext>,
    HttpRequestContext(request_ctx): HttpRequestContext,
    actor: UserActor,
    Path(organization_id): Path<String>,
    headers: HeaderMap,
    JsonBody(input): JsonBody<CreateInvitationDeliveryRequest>,
) -> Result<Json<CreateInvitationDeliveryResponse>, ApiErrorResponse> {
    let repository = PostgresOrganizationRepository::new(ctx.db.clone());
    require_permission(
        &repository,
        &organization_id,
        &actor.user_id,
        ORGANIZATION_INVITATIONS_MANAGE,
        &request_ctx,
    )
    .await?;
    let idempotency_key = headers
        .get(crate::notification::INVITATION_DELIVERY_IDEMPOTENCY_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            ApiErrorResponse::with_context(
                AppError::new(ErrorCode::Validation, "Idempotency-Key header is required"),
                &request_ctx,
            )
        })?;
    let config = ctx
        .config
        .module_local_config::<InvitationDeliveryConfig>(crate::module::MODULE_NAME)
        .map_err(|error| ApiErrorResponse::with_context(error, &request_ctx))?;
    let public_base_url = config.public_invitation_base_url.ok_or_else(|| {
        ApiErrorResponse::with_context(
            AppError::new(
                ErrorCode::Validation,
                "organization public_invitation_base_url is not configured",
            ),
            &request_ctx,
        )
    })?;
    let receipt = crate::notification::create_invitation_delivery(
        &repository,
        &request_ctx,
        &organization_id,
        &crate::notification::CreateInvitationDelivery {
            email: input.email,
            role_id: input.role_id,
            expires_at: input.expires_at,
            locale: input.locale,
            recipient_display_name: input.recipient_display_name,
        },
        idempotency_key,
        &public_base_url,
        ctx.clock.now(),
    )
    .await
    .map_err(|error| ApiErrorResponse::with_context(error, &request_ctx))?;
    Ok(json(CreateInvitationDeliveryResponse {
        invitation_id: receipt.invitation_id,
        organization_id: receipt.organization_id,
        notification_intent_id: receipt.notification_intent_id,
        notification_delivery_id: receipt.notification_delivery_id,
        delivery_status: receipt.delivery_status,
        expires_at: receipt.expires_at,
        idempotent_replay: receipt.idempotent_replay,
    }))
}

#[cfg(feature = "notification")]
#[derive(Debug, Default, serde::Deserialize)]
struct InvitationDeliveryConfig {
    public_invitation_base_url: Option<String>,
}

#[utoipa::path(
    post,
    path = "/v1/organizations",
    operation_id = "organization_create_organization",
    tag = "organization",
    request_body(content = CreateOrganizationRequest, content_type = "application/json"),
    responses(
        (status = 200, description = "Organization created", body = OrganizationResponse),
        (status = 401, description = "Authentication is required", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
async fn create_organization(
    State(ctx): State<AppContext>,
    HttpRequestContext(request_ctx): HttpRequestContext,
    actor: UserActor,
    JsonBody(input): JsonBody<CreateOrganizationRequest>,
) -> Result<Json<OrganizationResponse>, ApiErrorResponse> {
    let repository = PostgresOrganizationRepository::new(ctx.db.clone());
    let now = ctx.clock.now();
    #[cfg(feature = "audit-log")]
    let organization = repository
        .create_organization_with_owner_audited(
            &request_ctx,
            &input.name,
            &input.slug,
            &AuthUserId(actor.user_id),
            now,
        )
        .await
        .map_err(|error| ApiErrorResponse::with_context(error, &request_ctx))?;
    #[cfg(not(feature = "audit-log"))]
    let organization = repository
        .create_organization_with_owner(&input.name, &input.slug, &AuthUserId(actor.user_id), now)
        .await
        .map_err(|error| ApiErrorResponse::with_context(error, &request_ctx))?;
    Ok(json(organization_response(organization)))
}

#[utoipa::path(
    get,
    path = "/v1/organizations",
    operation_id = "organization_list_organizations",
    tag = "organization",
    responses(
        (status = 200, description = "Organizations listed", body = OrganizationListResponse),
        (status = 401, description = "Authentication is required", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
async fn list_organizations(
    State(ctx): State<AppContext>,
    HttpRequestContext(request_ctx): HttpRequestContext,
    actor: UserActor,
) -> Result<Json<OrganizationListResponse>, ApiErrorResponse> {
    let organizations = PostgresOrganizationRepository::new(ctx.db.clone())
        .list_user_organizations(&AuthUserId(actor.user_id))
        .await
        .map_err(|error| ApiErrorResponse::with_context(error, &request_ctx))?
        .into_iter()
        .map(organization_response)
        .collect();
    Ok(json(OrganizationListResponse { organizations }))
}

#[utoipa::path(
    get,
    path = "/v1/organizations/{id}/members",
    operation_id = "organization_list_members",
    tag = "organization",
    params(("id" = String, Path, description = "Organization id")),
    responses(
        (status = 200, description = "Members listed", body = MemberListResponse),
        (status = 403, description = "Organization permission is required", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
async fn list_members(
    State(ctx): State<AppContext>,
    HttpRequestContext(request_ctx): HttpRequestContext,
    actor: UserActor,
    Path(organization_id): Path<String>,
) -> Result<Json<MemberListResponse>, ApiErrorResponse> {
    let repository = PostgresOrganizationRepository::new(ctx.db.clone());
    require_permission(
        &repository,
        &organization_id,
        &actor.user_id,
        ORGANIZATION_READ,
        &request_ctx,
    )
    .await?;
    let members = repository
        .list_members(&organization_id)
        .await
        .map_err(|error| ApiErrorResponse::with_context(error, &request_ctx))?
        .into_iter()
        .map(member_response)
        .collect();
    Ok(json(MemberListResponse { members }))
}

#[utoipa::path(
    post,
    path = "/v1/organizations/{id}/invitations",
    operation_id = "organization_create_invitation",
    tag = "organization",
    params(("id" = String, Path, description = "Organization id")),
    request_body(content = CreateInvitationRequest, content_type = "application/json"),
    responses(
        (status = 200, description = "Invitation created", body = CreateInvitationResponse),
        (status = 403, description = "Organization permission is required", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
async fn create_invitation(
    State(ctx): State<AppContext>,
    HttpRequestContext(request_ctx): HttpRequestContext,
    actor: UserActor,
    Path(organization_id): Path<String>,
    JsonBody(input): JsonBody<CreateInvitationRequest>,
) -> Result<Json<CreateInvitationResponse>, ApiErrorResponse> {
    let repository = PostgresOrganizationRepository::new(ctx.db.clone());
    require_permission(
        &repository,
        &organization_id,
        &actor.user_id,
        ORGANIZATION_INVITATIONS_MANAGE,
        &request_ctx,
    )
    .await?;
    let now = ctx.clock.now();
    #[cfg(feature = "audit-log")]
    let created = repository
        .create_invitation_audited(
            &request_ctx,
            &organization_id,
            &input.email,
            &input.role_id,
            input.expires_at,
            now,
        )
        .await
        .map_err(|error| ApiErrorResponse::with_context(error, &request_ctx))?;
    #[cfg(not(feature = "audit-log"))]
    let created = repository
        .create_invitation(
            &organization_id,
            &input.email,
            &input.role_id,
            input.expires_at,
            now,
        )
        .await
        .map_err(|error| ApiErrorResponse::with_context(error, &request_ctx))?;
    Ok(json(CreateInvitationResponse {
        id: created.invitation.id,
        organization_id: created.invitation.organization_id,
        email: created.invitation.email,
        role_id: created.invitation.role_id,
        token: created.token,
        expires_at: created.invitation.expires_at,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/organization-invitations/{token}/accept",
    operation_id = "organization_accept_invitation",
    tag = "organization",
    params(("token" = String, Path, description = "Invitation token")),
    responses(
        (status = 200, description = "Invitation accepted", body = AcceptInvitationResponse),
        (status = 401, description = "Authentication is required", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
async fn accept_invitation(
    State(ctx): State<AppContext>,
    HttpRequestContext(request_ctx): HttpRequestContext,
    actor: UserActor,
    Path(token): Path<String>,
) -> Result<Json<AcceptInvitationResponse>, ApiErrorResponse> {
    let repository = PostgresOrganizationRepository::new(ctx.db.clone());
    let now = ctx.clock.now();
    #[cfg(feature = "audit-log")]
    let membership = repository
        .accept_invitation_audited(&request_ctx, &token, &AuthUserId(actor.user_id), now)
        .await
        .map_err(|error| ApiErrorResponse::with_context(error, &request_ctx))?;
    #[cfg(not(feature = "audit-log"))]
    let membership = repository
        .accept_invitation(&token, &AuthUserId(actor.user_id), now)
        .await
        .map_err(|error| ApiErrorResponse::with_context(error, &request_ctx))?;
    Ok(json(AcceptInvitationResponse {
        membership: member_response(membership),
    }))
}

async fn require_permission(
    repository: &PostgresOrganizationRepository,
    organization_id: &str,
    user_id: &str,
    permission: &str,
    request_ctx: &platform_core::RequestContext,
) -> Result<(), ApiErrorResponse> {
    let allowed = repository
        .has_permission(organization_id, &AuthUserId(user_id.to_owned()), permission)
        .await
        .map_err(|error| ApiErrorResponse::with_context(error, request_ctx))?;
    if allowed {
        return Ok(());
    }
    Err(ApiErrorResponse::with_context(
        AppError::new(ErrorCode::Forbidden, "Organization permission is required"),
        request_ctx,
    ))
}

fn organization_response(organization: Organization) -> OrganizationResponse {
    OrganizationResponse {
        id: organization.id,
        name: organization.name,
        slug: organization.slug,
        created_at: organization.created_at,
        updated_at: organization.updated_at,
        archived_at: organization.archived_at,
    }
}

fn member_response(membership: Membership) -> MemberResponse {
    MemberResponse {
        id: membership.id,
        organization_id: membership.organization_id,
        auth_user_id: membership.auth_user_id.0,
        role_id: membership.role_id,
        role_name: membership.role_name,
        created_at: membership.created_at,
        updated_at: membership.updated_at,
        removed_at: membership.removed_at,
    }
}
