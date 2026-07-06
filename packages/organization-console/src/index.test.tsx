import { describe, expect, test } from "vitest";

import {
  OrganizationConsolePage,
  organizationConsoleManifest,
  organizationConsoleModule,
} from ".";

describe("organization console package", () => {
  test("declares the organization console package contract and module export", () => {
    expect(organizationConsoleManifest).toMatchObject({
      exportName: "organizationConsoleModule",
      id: "organization",
      packageName: "@lenso/organization-console",
      source: "runtime_bundle",
      version: "workspace",
      surfaces: [
        {
          icon: "boxes",
          label: "Organizations",
          route: "/data/organization",
          surfaceName: "organizations",
        },
        {
          icon: "users",
          label: "Members",
          route: "/data/organization/members",
          surfaceName: "members",
        },
        {
          icon: "shield",
          label: "Roles",
          route: "/data/organization/roles",
          surfaceName: "roles",
        },
        {
          icon: "key-round",
          label: "Invitations",
          route: "/data/organization/invitations",
          surfaceName: "invitations",
        },
      ],
    });
    expect(organizationConsoleManifest.surfaces).toHaveLength(4);

    expect(organizationConsoleModule).toMatchObject({
      id: "organization",
      surfaces: [
        {
          label: "Organizations",
          path: "/data/organization",
        },
        {
          label: "Members",
          path: "/data/organization/members",
        },
        {
          label: "Roles",
          path: "/data/organization/roles",
        },
        {
          label: "Invitations",
          path: "/data/organization/invitations",
        },
      ],
    });
    expect(organizationConsoleModule.surfaces).toHaveLength(4);
    expect(OrganizationConsolePage).toBeTypeOf("function");
  });
});
