# Release process

The former `lenso-module-organization` line ended at its existing public crate
versions and tags. The default branch now owns vNext packages with different
identities; it must not republish or overwrite the legacy package.

The four Capability packages and the PostgreSQL Plugin are public release
artifacts. Publication is a separate, explicitly authorized operation. The
repository workflow is manual-only: it has no `main` push trigger and never
opens or updates a release PR. A normal contribution lands through the
candidate-first contract in [CONTRIBUTING.md](../CONTRIBUTING.md), not through
release-plz.

## Trusted Publishing boundaries

Before the first publication of a new crate name:

1. prove generated contract freshness, workspace tests, PostgreSQL acceptance,
   repository boundary, and independent package verification;
2. make the four Capability packages public before the implementation package;
3. allocate the name using crates.io's required one-time initial-publish
   process; Trusted Publishing cannot create a new crate name;
4. configure a crates.io Trusted Publisher for every published crate with owner
   `LioRael`, repository `lenso-organization-plugin`, and workflow
   `release-plz.yml`; and
5. run a separately authorized live workflow only after every crate has the
   matching publisher.

The workflow never accepts or falls back to a long-lived registry token.
Release-plz obtains a short-lived crates.io credential from GitHub OIDC, and a
live job has only the `id-token: write` permission needed for that exchange.
Repository write access, a landed commit, and a successful dry-run do not grant
publication authority.

## Controlled dispatch

Dispatch `.github/workflows/release-plz.yml` from `main` with:

- `mode=dry-run` (the only mode authorized for this rollout);
- the full landed `source_sha`;
- the exact successful candidate CI `run_id` and `run_attempt`; and
- the JSON `release_set` in the documented publication order.

The workflow verifies that `source_sha` is on `main` and that the exact
candidate `check` run succeeded for that SHA before installing release tooling
or invoking release-plz. Dry-run is read-only and does not create tags,
releases, release PRs, or uploads. This rollout must stop before `mode=publish`;
the live branch remains a guarded policy boundary for a future, separately
authorized operation.

Publish order, if live publication is separately authorized, is:

1. `lenso-capability-organization-admin`;
2. `lenso-capability-organization-directory`;
3. `lenso-capability-organization-membership`;
4. `lenso-capability-organization-membership-admin`; and
5. `lenso-organization-postgres-plugin`.

Do not use `--no-verify`, a long-lived registry token, or Git dependencies as a
publication shortcut.
