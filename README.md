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
- Optional atomic invitation plus transactional-email intent creation.
- Public helpers such as `has_permission` and `accept_invitation`.
- HTTP routes and schema-admin/admin-action surfaces.

## Business administration

Organization CRUD is a business administration surface, not a Lenso Console
System Plane capability. Install the Rust modules in the application that owns
the business administration experience:

```sh
lenso module install auth
lenso module install organization
```

The public HTTP and declarative administration contracts expose:

- Organizations: create and archive organizations.
- Members: inspect memberships, update member roles, and remove members.
- Roles: create custom roles and update role permission arrays. Seeded `owner`
  role is protected from permission edits.
- Invitations: create invitations, copy the raw token returned once, and revoke
  unaccepted invitations.

Security note: invitation tokens are stored only as hashes in Organization
tables. Administration responses never list `token_hash`. With the optional
`notification` feature, the token is passed directly into Notification's
protected immutable render snapshot inside the same database transaction and
is never returned by the invitation-delivery endpoint.

## Optional Notification Integration

Enable `notification` after installing `lenso-module-notification` to create an
invitation and its first transactional email intent atomically:

```toml
lenso-module-organization = { version = "0.1.4", features = ["notification"] }
lenso-module-notification = "0.1.0"
```

Configure `organization.public_invitation_base_url` and the Notification
snapshot protector, then call the idempotent delivery endpoint:

```sh
curl -X POST /v1/organizations/{organization_id}/invitation-deliveries \
  -H 'content-type: application/json' \
  -H 'idempotency-key: invite-member-2026-08-13' \
  -d '{"email":"member@example.com","role_id":"org_role_...","expires_at":"2026-08-14T00:00:00Z","locale":"en"}'
```

The response contains invitation, Notification intent, and delivery IDs plus
the initial delivery status. It never contains the invitation token or URL.
Reusing the key with identical input returns the same receipt; changed input
fails with a conflict. The legacy invitation creation endpoint remains for
applications that intentionally own delivery themselves.

## Optional Audit Log Integration

`organization` can write audit events when built with the `audit-log` feature
and installed beside `lenso-module-audit-log`.

```toml
lenso-module-organization = { version = "0.1.2", features = ["audit-log"] }
lenso-module-audit-log = "0.1.1"
```

The integration records organization-created, invitation-created, and
invitation-accepted events from the HTTP route path. Events use generic audit
scopes:

```text
scope_module = organization
scope_type = organization
scope_id = <organization id>
```

The audit module does not depend on organization internals. Publish
`lenso-module-audit-log` before publishing an `organization` release that enables
the feature.

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

The legacy invitation endpoint returns a token only on initial creation. The
invitation-delivery endpoint never returns one. Organization tables store only
`token_hash`; Notification stores the rendered invitation URL behind its
application-layer snapshot protector and excludes it from Console/API output.

## Development

```sh
cargo fmt --all --check
cargo test --locked -p lenso-module-organization
```

## Release

Release-plz opens a release pull request from changes merged to `main` and
publishes the crate through crates.io Trusted Publishing after that pull
request is merged. See [`docs/release-process.md`](docs/release-process.md) for
the repository-owned release contract.
