use crate::admin::{
    ARCHIVE_ORGANIZATION_ACTION, CREATE_INVITATION_ACTION, CREATE_ORGANIZATION_ACTION,
    CREATE_ROLE_ACTION, OrganizationAdminData, REMOVE_MEMBER_ACTION, REVOKE_INVITATION_ACTION,
    UPDATE_MEMBER_ROLE_ACTION, UPDATE_ROLE_PERMISSIONS_ACTION,
};
use crate::migrations::ORGANIZATION_MIGRATIONS;
use crate::repositories::PostgresOrganizationRepository;
use platform_core::AppContext;
use platform_http::ApiOpenApiRouter;
use platform_module::{
    AdminAction, AdminActionDangerLevel, AdminActionInputField, AdminActionInputSchema,
    AdminDeclarativeComponent, AdminDeclarativePage, AdminDeclarativeSection,
    AdminDeclarativeSurface, AdminSchema, ConsoleArea, ConsoleNavigation, ConsolePackage,
    ConsoleSurface, ConsoleWorkspaceRef, EntitySchema, FieldSchema, FieldType, HostLinkedModule,
    LinkedBinding, LinkedHttpContribution, Module, ModuleHttpMethod, ModuleHttpRoute,
    ModuleManifest,
};
use std::sync::Arc;

pub const MODULE_NAME: &str = "organization";
pub const AUTH_MODULE_DEPENDENCY: &str = "auth";
pub const ORGANIZATION_CONSOLE_PACKAGE: &str = "@lenso/organization-console";
pub const ORGANIZATION_CONSOLE_EXPORT: &str = "organizationConsoleModule";
pub const ORGANIZATION_READ: &str = "organization.read";
pub const ORGANIZATION_MANAGE: &str = "organization.manage";
pub const ORGANIZATION_MEMBERS_MANAGE: &str = "organization.members.manage";
pub const ORGANIZATION_ROLES_MANAGE: &str = "organization.roles.manage";
pub const ORGANIZATION_INVITATIONS_MANAGE: &str = "organization.invitations.manage";

pub fn capabilities() -> Vec<String> {
    vec![
        ORGANIZATION_READ.to_owned(),
        ORGANIZATION_MANAGE.to_owned(),
        ORGANIZATION_MEMBERS_MANAGE.to_owned(),
        ORGANIZATION_ROLES_MANAGE.to_owned(),
        ORGANIZATION_INVITATIONS_MANAGE.to_owned(),
    ]
}

pub fn http_routes() -> Vec<ModuleHttpRoute> {
    vec![
        route(
            ModuleHttpMethod::Post,
            "/v1/organizations",
            None,
            "Create Organization",
        ),
        route(
            ModuleHttpMethod::Get,
            "/v1/organizations",
            None,
            "List Organizations",
        ),
        route(
            ModuleHttpMethod::Get,
            "/v1/organizations/{id}/members",
            Some(ORGANIZATION_READ),
            "List Organization Members",
        ),
        route(
            ModuleHttpMethod::Post,
            "/v1/organizations/{id}/invitations",
            Some(ORGANIZATION_INVITATIONS_MANAGE),
            "Create Organization Invitation",
        ),
        route(
            ModuleHttpMethod::Post,
            "/v1/organization-invitations/{token}/accept",
            None,
            "Accept Organization Invitation",
        ),
    ]
}

pub fn organization_schema() -> AdminSchema {
    AdminSchema {
        entities: vec![
            EntitySchema {
                name: "organizations".to_owned(),
                label: "Organizations".to_owned(),
                read_capability: ORGANIZATION_READ.to_owned(),
                fields: vec![
                    field("id", "ID", FieldType::String, false),
                    field("name", "Name", FieldType::String, false),
                    field("slug", "Slug", FieldType::String, false),
                    field("created_at", "Created", FieldType::Timestamp, false),
                    field("updated_at", "Updated", FieldType::Timestamp, false),
                    field("archived_at", "Archived", FieldType::Timestamp, true),
                ],
            },
            EntitySchema {
                name: "roles".to_owned(),
                label: "Roles".to_owned(),
                read_capability: ORGANIZATION_ROLES_MANAGE.to_owned(),
                fields: vec![
                    field("id", "ID", FieldType::String, false),
                    field("organization_id", "Organization", FieldType::String, false),
                    field("name", "Name", FieldType::String, false),
                    field("permissions", "Permissions", FieldType::Json, false),
                    field("system_key", "System key", FieldType::String, true),
                    field("created_at", "Created", FieldType::Timestamp, false),
                    field("updated_at", "Updated", FieldType::Timestamp, false),
                ],
            },
            EntitySchema {
                name: "memberships".to_owned(),
                label: "Memberships".to_owned(),
                read_capability: ORGANIZATION_MEMBERS_MANAGE.to_owned(),
                fields: vec![
                    field("id", "ID", FieldType::String, false),
                    field("organization_id", "Organization", FieldType::String, false),
                    field("auth_user_id", "Auth user", FieldType::String, false),
                    field("role_id", "Role", FieldType::String, false),
                    field("role_name", "Role name", FieldType::String, true),
                    field("created_at", "Created", FieldType::Timestamp, false),
                    field("updated_at", "Updated", FieldType::Timestamp, false),
                    field("removed_at", "Removed", FieldType::Timestamp, true),
                ],
            },
            EntitySchema {
                name: "invitations".to_owned(),
                label: "Invitations".to_owned(),
                read_capability: ORGANIZATION_INVITATIONS_MANAGE.to_owned(),
                fields: vec![
                    field("id", "ID", FieldType::String, false),
                    field("organization_id", "Organization", FieldType::String, false),
                    field("email", "Email", FieldType::String, false),
                    field("role_id", "Role", FieldType::String, false),
                    field("expires_at", "Expires", FieldType::Timestamp, false),
                    field("created_at", "Created", FieldType::Timestamp, false),
                    field("updated_at", "Updated", FieldType::Timestamp, false),
                    field("accepted_at", "Accepted", FieldType::Timestamp, true),
                    field("revoked_at", "Revoked", FieldType::Timestamp, true),
                ],
            },
        ],
    }
}

pub fn admin_surface() -> AdminDeclarativeSurface {
    AdminDeclarativeSurface {
        pages: vec![
            page("organizations", "Organizations"),
            page("roles", "Roles"),
            page("memberships", "Memberships"),
            page("invitations", "Invitations"),
        ],
        actions: vec![
            create_organization_action(),
            string_action(
                ARCHIVE_ORGANIZATION_ACTION,
                "Archive organization",
                ORGANIZATION_MANAGE,
                "organization_id",
                "Organization",
                AdminActionDangerLevel::High,
            ),
            create_role_action(),
            update_role_permissions_action(),
            create_invitation_action(),
            string_pair_action(
                UPDATE_MEMBER_ROLE_ACTION,
                "Update member role",
                ORGANIZATION_MEMBERS_MANAGE,
                "membership_id",
                "Membership",
                "role_id",
                "Role",
                AdminActionDangerLevel::Medium,
            ),
            string_action(
                REVOKE_INVITATION_ACTION,
                "Revoke invitation",
                ORGANIZATION_INVITATIONS_MANAGE,
                "invitation_id",
                "Invitation",
                AdminActionDangerLevel::Medium,
            ),
            string_action(
                REMOVE_MEMBER_ACTION,
                "Remove member",
                ORGANIZATION_MEMBERS_MANAGE,
                "membership_id",
                "Membership",
                AdminActionDangerLevel::Medium,
            ),
        ],
        fallback_schema: Some(organization_schema()),
    }
}

fn organization_workspace() -> ConsoleWorkspaceRef {
    ConsoleWorkspaceRef {
        id: "organization".to_owned(),
        label: "Organization".to_owned(),
        icon: Some("boxes".to_owned()),
    }
}

pub fn console_surfaces() -> Vec<ConsoleSurface> {
    vec![
        console_surface(
            "organizations",
            "Organizations",
            "/data/organization",
            "boxes",
            ORGANIZATION_READ,
            70,
        ),
        console_surface(
            "members",
            "Members",
            "/data/organization/members",
            "users",
            ORGANIZATION_MEMBERS_MANAGE,
            80,
        ),
        console_surface(
            "roles",
            "Roles",
            "/data/organization/roles",
            "shield",
            ORGANIZATION_ROLES_MANAGE,
            90,
        ),
        console_surface(
            "invitations",
            "Invitations",
            "/data/organization/invitations",
            "key-round",
            ORGANIZATION_INVITATIONS_MANAGE,
            100,
        ),
    ]
}

fn console_surface(
    name: &str,
    label: &str,
    route: &str,
    icon: &str,
    capability: &str,
    order: i32,
) -> ConsoleSurface {
    ConsoleSurface {
        name: name.to_owned(),
        label: label.to_owned(),
        area: ConsoleArea::Data,
        route: route.to_owned(),
        package: ConsolePackage {
            name: ORGANIZATION_CONSOLE_PACKAGE.to_owned(),
            export: ORGANIZATION_CONSOLE_EXPORT.to_owned(),
        },
        icon: Some(icon.to_owned()),
        required_capabilities: vec![capability.to_owned()],
        navigation: Some(ConsoleNavigation {
            workspace: organization_workspace(),
            group: None,
            order: Some(order),
        }),
    }
}

pub fn manifest() -> ModuleManifest {
    ModuleManifest::builder(MODULE_NAME)
        .dependencies(vec![AUTH_MODULE_DEPENDENCY.to_owned()])
        .capabilities(capabilities())
        .http_routes(http_routes())
        .declarative_admin(admin_surface())
        .console(console_surfaces())
        .build()
}

pub fn merge_http(base: ApiOpenApiRouter) -> ApiOpenApiRouter {
    base.merge(crate::routes::router())
}

pub fn binding() -> LinkedBinding {
    LinkedBinding::builder()
        .http(LinkedHttpContribution {
            public_prefixes: &["/v1/organizations", "/v1/organization-invitations"],
            merge: merge_http,
        })
        .build()
}

pub fn module(ctx: &AppContext) -> Module {
    let repository = Arc::new(PostgresOrganizationRepository::new(ctx.db.clone()));
    let admin = Arc::new(OrganizationAdminData::new(repository));
    Module::linked(manifest(), binding())
        .with_admin_data(admin.clone())
        .with_admin_actions(admin)
}

pub fn linked_module() -> HostLinkedModule {
    HostLinkedModule::linked(MODULE_NAME, manifest, module, ORGANIZATION_MIGRATIONS)
        .with_http_binding(binding)
}

fn route(
    method: ModuleHttpMethod,
    path: &str,
    capability: Option<&str>,
    display_name: &str,
) -> ModuleHttpRoute {
    ModuleHttpRoute {
        method,
        path: path.to_owned(),
        capability: capability.map(ToOwned::to_owned),
        display_name: Some(display_name.to_owned()),
        story_title: Some(display_name.to_owned()),
    }
}

fn field(name: &str, label: &str, field_type: FieldType, nullable: bool) -> FieldSchema {
    FieldSchema {
        name: name.to_owned(),
        label: label.to_owned(),
        field_type,
        nullable,
    }
}

fn page(entity: &str, label: &str) -> AdminDeclarativePage {
    AdminDeclarativePage {
        name: entity.to_owned(),
        label: label.to_owned(),
        sections: vec![AdminDeclarativeSection {
            name: entity.to_owned(),
            label: label.to_owned(),
            component: AdminDeclarativeComponent::EntityTable {
                entity: entity.to_owned(),
            },
        }],
    }
}

fn create_organization_action() -> AdminAction {
    AdminAction {
        name: CREATE_ORGANIZATION_ACTION.to_owned(),
        label: "Create organization".to_owned(),
        capability: ORGANIZATION_MANAGE.to_owned(),
        input_schema: Some(AdminActionInputSchema {
            fields: vec![
                input("name", "Name", FieldType::String, true, None),
                input("slug", "Slug", FieldType::String, true, None),
                input(
                    "owner_auth_user_id",
                    "Owner auth user",
                    FieldType::String,
                    true,
                    None,
                ),
            ],
        }),
        confirmation: None,
        danger_level: AdminActionDangerLevel::Low,
    }
}

fn create_role_action() -> AdminAction {
    AdminAction {
        name: CREATE_ROLE_ACTION.to_owned(),
        label: "Create role".to_owned(),
        capability: ORGANIZATION_ROLES_MANAGE.to_owned(),
        input_schema: Some(AdminActionInputSchema {
            fields: vec![
                input(
                    "organization_id",
                    "Organization",
                    FieldType::String,
                    true,
                    None,
                ),
                input("name", "Name", FieldType::String, true, None),
                input(
                    "permissions",
                    "Permissions",
                    FieldType::Json,
                    true,
                    Some("JSON array of permission strings".to_owned()),
                ),
            ],
        }),
        confirmation: None,
        danger_level: AdminActionDangerLevel::Low,
    }
}

fn update_role_permissions_action() -> AdminAction {
    AdminAction {
        name: UPDATE_ROLE_PERMISSIONS_ACTION.to_owned(),
        label: "Update role permissions".to_owned(),
        capability: ORGANIZATION_ROLES_MANAGE.to_owned(),
        input_schema: Some(AdminActionInputSchema {
            fields: vec![
                input("role_id", "Role", FieldType::String, true, None),
                input(
                    "permissions",
                    "Permissions",
                    FieldType::Json,
                    true,
                    Some("JSON array of permission strings".to_owned()),
                ),
            ],
        }),
        confirmation: None,
        danger_level: AdminActionDangerLevel::Medium,
    }
}

fn create_invitation_action() -> AdminAction {
    AdminAction {
        name: CREATE_INVITATION_ACTION.to_owned(),
        label: "Create invitation".to_owned(),
        capability: ORGANIZATION_INVITATIONS_MANAGE.to_owned(),
        input_schema: Some(AdminActionInputSchema {
            fields: vec![
                input(
                    "organization_id",
                    "Organization",
                    FieldType::String,
                    true,
                    None,
                ),
                input("email", "Email", FieldType::String, true, None),
                input("role_id", "Role", FieldType::String, true, None),
                input(
                    "expires_at",
                    "Expires",
                    FieldType::Timestamp,
                    true,
                    Some("RFC3339 timestamp".to_owned()),
                ),
            ],
        }),
        confirmation: None,
        danger_level: AdminActionDangerLevel::Low,
    }
}

fn string_action(
    name: &str,
    label: &str,
    capability: &str,
    input_name: &str,
    input_label: &str,
    danger_level: AdminActionDangerLevel,
) -> AdminAction {
    AdminAction {
        name: name.to_owned(),
        label: label.to_owned(),
        capability: capability.to_owned(),
        input_schema: Some(AdminActionInputSchema {
            fields: vec![input(
                input_name,
                input_label,
                FieldType::String,
                true,
                None,
            )],
        }),
        confirmation: None,
        danger_level,
    }
}

fn string_pair_action(
    name: &str,
    label: &str,
    capability: &str,
    first_name: &str,
    first_label: &str,
    second_name: &str,
    second_label: &str,
    danger_level: AdminActionDangerLevel,
) -> AdminAction {
    AdminAction {
        name: name.to_owned(),
        label: label.to_owned(),
        capability: capability.to_owned(),
        input_schema: Some(AdminActionInputSchema {
            fields: vec![
                input(first_name, first_label, FieldType::String, true, None),
                input(second_name, second_label, FieldType::String, true, None),
            ],
        }),
        confirmation: None,
        danger_level,
    }
}

fn input(
    name: &str,
    label: &str,
    field_type: FieldType,
    required: bool,
    description: Option<String>,
) -> AdminActionInputField {
    AdminActionInputField {
        name: name.to_owned(),
        label: label.to_owned(),
        field_type,
        required,
        description,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_module::{AdminSurface, ModuleManifestLintSeverity, ModuleSource};

    #[test]
    fn manifest_declares_organization_surfaces() {
        let manifest = manifest();

        assert_eq!(manifest.name, MODULE_NAME);
        assert_eq!(manifest.dependencies, vec![AUTH_MODULE_DEPENDENCY]);
        assert_eq!(manifest.capabilities, capabilities());
        assert_eq!(manifest.http_routes, http_routes());
        assert_eq!(
            manifest.admin,
            Some(AdminSurface::DeclarativeCustom(admin_surface()))
        );
        assert_eq!(manifest.console, console_surfaces());

        let lints = platform_module::lint_module_manifest(ModuleSource::Linked, &manifest);
        assert!(
            lints
                .iter()
                .all(|lint| lint.severity == ModuleManifestLintSeverity::Ok),
            "organization manifest should not have warning/error lints: {lints:?}"
        );
    }

    #[test]
    fn manifest_declares_console_surfaces() {
        let manifest = manifest();

        assert_eq!(manifest.console, console_surfaces());
        assert_eq!(manifest.console.len(), 4);
        assert_eq!(manifest.console[0].route, "/data/organization");
        assert_eq!(manifest.console[1].route, "/data/organization/members");
        assert_eq!(manifest.console[2].route, "/data/organization/roles");
        assert_eq!(manifest.console[3].route, "/data/organization/invitations");
        assert_eq!(
            manifest.console[0].package.name,
            "@lenso/organization-console"
        );
        assert_eq!(manifest.console[0].package.export, "organizationConsoleModule");
    }

    #[test]
    fn linked_module_exposes_http_binding() {
        let linked = linked_module();

        assert_eq!(linked.module_name, MODULE_NAME);
        assert!(linked.load.is_some());
        assert!(linked.http_binding.is_some());
        assert_eq!(linked.migrations.len(), ORGANIZATION_MIGRATIONS.len());
        assert_eq!(linked.migrations[0].name, ORGANIZATION_MIGRATIONS[0].name);
    }
}
