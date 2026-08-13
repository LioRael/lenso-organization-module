use platform_core::Migration;

pub const ORGANIZATION_MIGRATIONS: &[Migration] = &[
    Migration {
        name: "organization/0001_create_organization_schema",
        sql: include_str!("../migrations/0001_create_organization_schema.sql"),
    },
    Migration {
        name: "organization/0002_create_invitation_delivery_requests",
        sql: include_str!("../migrations/0002_create_invitation_delivery_requests.sql"),
    },
];
