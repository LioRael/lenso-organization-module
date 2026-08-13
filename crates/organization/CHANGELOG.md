## lenso-module-organization@0.1.4

### Added

- Add an optional Notification integration that creates organization
  invitations and transactional email intents atomically behind an
  `Idempotency-Key` contract.
- Add the `/v1/organizations/{id}/invitation-deliveries` endpoint with a
  token-free receipt containing the stable invitation, intent, and delivery
  identities.

### Security

- Keep the raw invitation token inside the atomic render path and remove the
  prepared-delivery token accessor from the module surface.

## lenso-module-organization@0.1.2

### Changed

Remove the retired same-origin Console package and keep organization CRUD in
the business administration plane outside Lenso Console.
