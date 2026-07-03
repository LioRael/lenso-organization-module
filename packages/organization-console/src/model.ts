import type { ConsoleAdminRecord } from "@lenso/runtime-console-api";

const fieldText = (value: unknown): string =>
  typeof value === "string" && value.length > 0 ? value : "-";

const fieldStringArray = (value: unknown): string[] =>
  Array.isArray(value)
    ? value.filter((item): item is string => typeof item === "string")
    : [];

export type OrganizationRow = {
  archivedAt: string;
  createdAt: string;
  id: string;
  name: string;
  slug: string;
  status: "active" | "archived";
  updatedAt: string;
};

export type OrganizationSummary = {
  active: number;
  archived: number;
  total: number;
};

export type RoleRow = {
  createdAt: string;
  editable: boolean;
  id: string;
  name: string;
  organizationId: string;
  permissionCount: number;
  permissions: string[];
  systemKey: string;
  systemLabel: string;
  updatedAt: string;
};

export type MembershipRow = {
  authUserId: string;
  createdAt: string;
  id: string;
  organizationId: string;
  removedAt: string;
  roleId: string;
  roleName: string;
  status: "active" | "removed";
  updatedAt: string;
};

export type InvitationRow = {
  acceptedAt: string;
  createdAt: string;
  email: string;
  expiresAt: string;
  id: string;
  organizationId: string;
  revokedAt: string;
  roleId: string;
  status: "accepted" | "active" | "expired" | "revoked";
  updatedAt: string;
};

export type InvitationSummary = {
  accepted: number;
  active: number;
  expired: number;
  revoked: number;
  total: number;
};

export const DEFAULT_ROLE_PERMISSIONS = [
  "organization.read",
  "organization.members.manage",
  "organization.invitations.manage",
] as const;

export const organizationRows = (
  records: readonly ConsoleAdminRecord[]
): OrganizationRow[] =>
  records.map((record) => {
    const archivedAt = fieldText(record.archived_at);
    return {
      archivedAt,
      createdAt: fieldText(record.created_at),
      id: fieldText(record.id),
      name: fieldText(record.name),
      slug: fieldText(record.slug),
      status: archivedAt === "-" ? "active" : "archived",
      updatedAt: fieldText(record.updated_at),
    };
  });

export const organizationSummary = (
  rows: readonly OrganizationRow[]
): OrganizationSummary =>
  rows.reduce(
    (summary, row) => {
      summary.total += 1;
      summary[row.status] += 1;
      return summary;
    },
    { active: 0, archived: 0, total: 0 }
  );

export const roleRows = (records: readonly ConsoleAdminRecord[]): RoleRow[] =>
  records.map((record) => {
    const systemKey = fieldText(record.system_key);
    const permissions = fieldStringArray(record.permissions);
    return {
      createdAt: fieldText(record.created_at),
      editable: systemKey !== "owner",
      id: fieldText(record.id),
      name: fieldText(record.name),
      organizationId: fieldText(record.organization_id),
      permissionCount: permissions.length,
      permissions,
      systemKey,
      systemLabel: systemKey === "-" ? "custom" : systemKey,
      updatedAt: fieldText(record.updated_at),
    };
  });

export const rolesForOrganization = (
  roles: readonly RoleRow[],
  organizationId: string | null
): RoleRow[] =>
  organizationId
    ? roles.filter((role) => role.organizationId === organizationId)
    : [...roles];

export const membershipRows = (
  records: readonly ConsoleAdminRecord[]
): MembershipRow[] =>
  records.map((record) => {
    const removedAt = fieldText(record.removed_at);
    return {
      authUserId: fieldText(record.auth_user_id),
      createdAt: fieldText(record.created_at),
      id: fieldText(record.id),
      organizationId: fieldText(record.organization_id),
      removedAt,
      roleId: fieldText(record.role_id),
      roleName: fieldText(record.role_name),
      status: removedAt === "-" ? "active" : "removed",
      updatedAt: fieldText(record.updated_at),
    };
  });

export const invitationRows = (
  records: readonly ConsoleAdminRecord[],
  now = new Date()
): InvitationRow[] =>
  records.map((record) => {
    const acceptedAt = fieldText(record.accepted_at);
    const expiresAt = fieldText(record.expires_at);
    const revokedAt = fieldText(record.revoked_at);
    return {
      acceptedAt,
      createdAt: fieldText(record.created_at),
      email: fieldText(record.email),
      expiresAt,
      id: fieldText(record.id),
      organizationId: fieldText(record.organization_id),
      revokedAt,
      roleId: fieldText(record.role_id),
      status: invitationStatus(expiresAt, acceptedAt, revokedAt, now),
      updatedAt: fieldText(record.updated_at),
    };
  });

export const invitationSummary = (
  rows: readonly InvitationRow[]
): InvitationSummary =>
  rows.reduce(
    (summary, row) => {
      summary.total += 1;
      summary[row.status] += 1;
      return summary;
    },
    { accepted: 0, active: 0, expired: 0, revoked: 0, total: 0 }
  );

function invitationStatus(
  expiresAt: string,
  acceptedAt: string,
  revokedAt: string,
  now: Date
): InvitationRow["status"] {
  if (acceptedAt !== "-") {
    return "accepted";
  }
  if (revokedAt !== "-") {
    return "revoked";
  }
  const expiresMs = Date.parse(expiresAt);
  return Number.isFinite(expiresMs) && expiresMs <= now.getTime()
    ? "expired"
    : "active";
}

export const membershipsForOrganization = (
  rows: readonly MembershipRow[],
  organizationId: string | null
): MembershipRow[] =>
  organizationId
    ? rows.filter((row) => row.organizationId === organizationId)
    : [...rows];

export const invitationsForOrganization = (
  rows: readonly InvitationRow[],
  organizationId: string | null
): InvitationRow[] =>
  organizationId
    ? rows.filter((row) => row.organizationId === organizationId)
    : [...rows];

export function buildPermissionInput(value: string): string[] {
  const permissions: string[] = [];
  for (const line of value.split(/\r?\n|,/)) {
    const permission = line.trim();
    if (permission.length > 0 && !permissions.includes(permission)) {
      permissions.push(permission);
    }
  }
  return permissions;
}
