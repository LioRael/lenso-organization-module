# Release Process

This repository independently releases the `lenso-module-organization` Rust
crate. It is not coordinated by a repository-wide release plan.

## Rust crate

Release-plz runs on pushes to `main`:

1. `release-pr` opens or updates a release pull request for the crate.
2. `release` publishes the version from a merged release pull request through
   crates.io Trusted Publishing and creates the `<crate>@<version>` tag.

The crates.io registry is the source of truth for versions already published.
Existing public versions, tags, and changelogs are not rewritten. No long-lived
`CARGO_REGISTRY_TOKEN` is used by the workflow.

Before enabling the first live publish, configure a crates.io Trusted Publisher
for `lenso-module-organization` and verify that the repository and workflow
claims match `.github/workflows/release-plz.yml`.

The optional `audit-log` feature depends on the independently published
`lenso-module-audit-log` crate. Publish or update that dependency before an
organization release that enables the feature; this is a dependency constraint,
not a shared release plan.

## Local checks

```sh
cargo fmt --all --check
cargo test --locked -p lenso-module-organization
cargo test --locked -p lenso-module-organization --features audit-log
cargo package --locked -p lenso-module-organization --allow-dirty
cargo publish --dry-run --locked -p lenso-module-organization --allow-dirty
```

Cross-repository compatibility is proven by SemVer requirements, contracts,
dependency updates, and focused integration checks. Do not restore a central
publisher, shadow registry, nonce, or global release channel to repair a failed
publication.
