CREATE TABLE organizations (
    organization_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1 CHECK(active IN (0,1)),
    revision INTEGER NOT NULL DEFAULT 1 CHECK(revision > 0)
) STRICT;
CREATE UNIQUE INDEX active_organization_slug ON organizations(slug) WHERE active=1;
CREATE TABLE organization_memberships (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    membership_id TEXT NOT NULL UNIQUE,
    organization_id TEXT NOT NULL REFERENCES organizations(organization_id),
    subject TEXT NOT NULL,
    is_owner INTEGER NOT NULL CHECK(is_owner IN (0,1)),
    active INTEGER NOT NULL DEFAULT 1 CHECK(active IN (0,1)),
    revision INTEGER NOT NULL DEFAULT 1 CHECK(revision > 0)
) STRICT;
CREATE UNIQUE INDEX active_organization_subject ON organization_memberships(organization_id,subject) WHERE active=1;
CREATE TABLE organization_receipts (
    caller TEXT NOT NULL COLLATE BINARY,
    family TEXT NOT NULL CHECK(family IN ('creation','membership')),
    idempotency_key TEXT NOT NULL,
    intent TEXT NOT NULL,
    response TEXT NOT NULL,
    PRIMARY KEY(caller,family,idempotency_key)
) STRICT;
CREATE TABLE organization_operation (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1),
    caller TEXT,
    family TEXT NOT NULL,
    request TEXT NOT NULL,
    intent TEXT NOT NULL,
    error TEXT,
    response TEXT,
    fresh INTEGER NOT NULL DEFAULT 0 CHECK(fresh IN (0,1)),
    membership_id TEXT,
    organization_id TEXT,
    revision INTEGER CHECK(revision > 0)
) STRICT;
