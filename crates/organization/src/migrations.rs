use platform_core::Migration;

pub const ORGANIZATION_MIGRATIONS: &[Migration] = &[Migration {
    name: "organization/0001_create_organization_schema",
    sql: include_str!("../migrations/0001_create_organization_schema.sql"),
}];
