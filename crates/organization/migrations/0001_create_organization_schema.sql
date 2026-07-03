create schema if not exists organization;

create table if not exists organization.organizations (
    id text primary key,
    name text not null,
    slug text not null,
    created_at timestamptz not null,
    updated_at timestamptz not null,
    archived_at timestamptz,
    constraint organizations_name_not_empty check (length(name) > 0),
    constraint organizations_slug_not_empty check (length(slug) > 0)
);

create unique index if not exists organizations_active_slug_key
    on organization.organizations (slug)
    where archived_at is null;

create table if not exists organization.roles (
    id text primary key,
    organization_id text not null references organization.organizations(id) on delete cascade,
    name text not null,
    permissions jsonb not null,
    system_key text,
    created_at timestamptz not null,
    updated_at timestamptz not null,
    constraint roles_name_not_empty check (length(name) > 0),
    constraint roles_permissions_array check (jsonb_typeof(permissions) = 'array')
);

create unique index if not exists roles_organization_name_key
    on organization.roles (organization_id, name);

create table if not exists organization.memberships (
    id text primary key,
    organization_id text not null references organization.organizations(id) on delete cascade,
    auth_user_id text not null references auth.users(id) on delete cascade,
    role_id text not null references organization.roles(id) on delete restrict,
    created_at timestamptz not null,
    updated_at timestamptz not null,
    removed_at timestamptz
);

create index if not exists memberships_auth_user_id_idx
    on organization.memberships (auth_user_id);

create unique index if not exists memberships_active_user_org_key
    on organization.memberships (organization_id, auth_user_id)
    where removed_at is null;

create table if not exists organization.invitations (
    id text primary key,
    organization_id text not null references organization.organizations(id) on delete cascade,
    email text not null,
    role_id text not null references organization.roles(id) on delete restrict,
    token_hash text not null,
    expires_at timestamptz not null,
    created_at timestamptz not null,
    updated_at timestamptz not null,
    accepted_at timestamptz,
    revoked_at timestamptz,
    constraint invitations_email_not_empty check (length(email) > 0),
    constraint invitations_token_hash_not_empty check (length(token_hash) > 0)
);

create index if not exists invitations_organization_id_idx
    on organization.invitations (organization_id);

create unique index if not exists invitations_active_token_hash_key
    on organization.invitations (token_hash)
    where accepted_at is null and revoked_at is null;
