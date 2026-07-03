import { describe, expect, test } from "vitest";

import {
  buildPermissionInput,
  invitationRows,
  invitationSummary,
  membershipRows,
  organizationRows,
  organizationSummary,
  roleRows,
  rolesForOrganization,
} from "./model";

describe("organization console model", () => {
  test("formats organization rows and summary", () => {
    const rows = organizationRows([
      {
        archived_at: null,
        created_at: "2026-07-03T01:00:00.000Z",
        id: "org_active",
        name: "Acme",
        slug: "acme",
        updated_at: "2026-07-03T02:00:00.000Z",
      },
      {
        archived_at: "2026-07-03T03:00:00.000Z",
        created_at: "2026-07-02T01:00:00.000Z",
        id: "org_archived",
        name: "Old",
        slug: "old",
        updated_at: "2026-07-03T03:00:00.000Z",
      },
    ]);

    expect(rows.map((row) => row.status)).toEqual(["active", "archived"]);
    expect(organizationSummary(rows)).toEqual({
      active: 1,
      archived: 1,
      total: 2,
    });
  });

  test("formats roles and filters by organization", () => {
    const roles = roleRows([
      {
        created_at: "2026-07-03T01:00:00.000Z",
        id: "role_owner",
        name: "owner",
        organization_id: "org_1",
        permissions: ["organization.read", "organization.manage"],
        system_key: "owner",
        updated_at: "2026-07-03T01:00:00.000Z",
      },
      {
        created_at: "2026-07-03T01:00:00.000Z",
        id: "role_custom",
        name: "billing",
        organization_id: "org_2",
        permissions: ["billing.read"],
        system_key: null,
        updated_at: "2026-07-03T01:00:00.000Z",
      },
    ]);

    expect(roles[0]).toMatchObject({
      editable: false,
      permissionCount: 2,
      systemLabel: "owner",
    });
    expect(roles[1]).toMatchObject({
      editable: true,
      permissionCount: 1,
      systemLabel: "custom",
    });
    expect(rolesForOrganization(roles, "org_2").map((role) => role.id)).toEqual([
      "role_custom",
    ]);
  });

  test("formats membership and invitation statuses", () => {
    const members = membershipRows([
      {
        auth_user_id: "usr_active",
        created_at: "2026-07-03T01:00:00.000Z",
        id: "mem_active",
        organization_id: "org_1",
        removed_at: null,
        role_id: "role_member",
        role_name: "member",
        updated_at: "2026-07-03T01:00:00.000Z",
      },
      {
        auth_user_id: "usr_removed",
        created_at: "2026-07-03T01:00:00.000Z",
        id: "mem_removed",
        organization_id: "org_1",
        removed_at: "2026-07-03T04:00:00.000Z",
        role_id: "role_member",
        role_name: "member",
        updated_at: "2026-07-03T04:00:00.000Z",
      },
    ]);
    expect(members.map((row) => row.status)).toEqual(["active", "removed"]);

    const invites = invitationRows(
      [
        {
          accepted_at: null,
          created_at: "2026-07-03T01:00:00.000Z",
          email: "active@example.com",
          expires_at: "2026-07-04T01:00:00.000Z",
          id: "inv_active",
          organization_id: "org_1",
          revoked_at: null,
          role_id: "role_member",
          updated_at: "2026-07-03T01:00:00.000Z",
        },
        {
          accepted_at: null,
          created_at: "2026-07-03T01:00:00.000Z",
          email: "expired@example.com",
          expires_at: "2026-07-02T01:00:00.000Z",
          id: "inv_expired",
          organization_id: "org_1",
          revoked_at: null,
          role_id: "role_member",
          updated_at: "2026-07-03T01:00:00.000Z",
        },
      ],
      new Date("2026-07-03T12:00:00.000Z")
    );

    expect(invites.map((row) => row.status)).toEqual(["active", "expired"]);
    expect(invitationSummary(invites)).toEqual({
      accepted: 0,
      active: 1,
      expired: 1,
      revoked: 0,
      total: 2,
    });
  });

  test("builds unique permission input from textarea text", () => {
    expect(
      buildPermissionInput(" organization.read\nbilling.read\norganization.read ")
    ).toEqual(["organization.read", "billing.read"]);
  });
});
