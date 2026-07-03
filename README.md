# Lenso Organization Module

First-party Lenso organization, role, membership, and invitation module.

The module is intentionally separate from `auth`: auth owns actor identity and
sessions, while `organization` owns reusable SaaS access structure around those
auth users.

## Package

- Rust: `lenso-module-organization`

## Install In A Lenso Host

Install auth first, then organization:

```sh
lenso module install auth
lenso module install organization
```

Or compose manually:

```rust
use lenso::host::prelude::*;

pub fn host_composition() -> HostComposition {
    HostBuilder::new()
        .linked_module(builtins::auth())
        .linked_module(organization::module::linked_module())
        .build()
}
```

## What It Provides

- Organizations with slugs and archive timestamps.
- Configurable per-organization roles with module-local permission strings.
- Memberships from `auth.users.id` to organizations.
- Invitation records with hashed tokens, expiration, acceptance, and revocation.
- Public helpers such as `has_permission` and `accept_invitation`.
- HTTP routes and schema-admin/admin-action surfaces.

## Basic Usage

Create an organization and make an auth user its owner:

```rust
let organization = organization::public::create_organization_with_owner(
    &ctx.db,
    "Acme",
    "acme",
    &auth::public::AuthUserId("usr_owner".to_owned()),
    ctx.clock.now(),
)
.await?;
```

Invite and accept members through the HTTP surface:

```sh
curl -X POST /v1/organizations/{organization_id}/invitations \
  -H 'content-type: application/json' \
  -d '{"email":"member@example.com","role_id":"org_role_...","expires_at":"2026-07-04T00:00:00Z"}'

curl -X POST /v1/organization-invitations/{token}/accept
```

Other modules should use the helper instead of reading role JSON directly:

```rust
let allowed = organization::public::has_permission(
    &ctx.db,
    &organization.id,
    &auth::public::AuthUserId("usr_owner".to_owned()),
    "organization.members.manage",
)
.await?;
```

## Security Notes

Invitation tokens are returned only when an invitation is created. The database
stores only `token_hash`. This module does not send email; callers should
deliver invitation URLs through their own mail or notification system.

## Development

```sh
cargo fmt --all --check
cargo test --locked -p lenso-module-organization
```
