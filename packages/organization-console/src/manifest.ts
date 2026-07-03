import { defineConsolePackageManifest } from "@lenso/runtime-console-api";

import consoleSurface from "../console-surface.json";

const consoleSurfaceContract = consoleSurface as unknown as {
  readonly exportName: "organizationConsoleModule";
  readonly id: "organization";
  readonly packageName: "@lenso/organization-console";
  readonly source: "runtime_bundle";
  readonly surfaces: readonly [
    {
      readonly area: "data";
      readonly icon: "boxes";
      readonly label: "Organizations";
      readonly navigation: {
        readonly order: 70;
        readonly workspace: {
          readonly icon: "boxes";
          readonly id: "organization";
          readonly label: "Organization";
        };
      };
      readonly requiredCapabilities: readonly ["organization.read"];
      readonly route: "/data/organization";
      readonly surfaceName: "organizations";
    },
    {
      readonly area: "data";
      readonly icon: "users";
      readonly label: "Members";
      readonly navigation: {
        readonly order: 80;
        readonly workspace: {
          readonly icon: "boxes";
          readonly id: "organization";
          readonly label: "Organization";
        };
      };
      readonly requiredCapabilities: readonly ["organization.members.manage"];
      readonly route: "/data/organization/members";
      readonly surfaceName: "members";
    },
    {
      readonly area: "data";
      readonly icon: "shield";
      readonly label: "Roles";
      readonly navigation: {
        readonly order: 90;
        readonly workspace: {
          readonly icon: "boxes";
          readonly id: "organization";
          readonly label: "Organization";
        };
      };
      readonly requiredCapabilities: readonly ["organization.roles.manage"];
      readonly route: "/data/organization/roles";
      readonly surfaceName: "roles";
    },
    {
      readonly area: "data";
      readonly icon: "key-round";
      readonly label: "Invitations";
      readonly navigation: {
        readonly order: 100;
        readonly workspace: {
          readonly icon: "boxes";
          readonly id: "organization";
          readonly label: "Organization";
        };
      };
      readonly requiredCapabilities: readonly ["organization.invitations.manage"];
      readonly route: "/data/organization/invitations";
      readonly surfaceName: "invitations";
    },
  ];
  readonly version: "workspace";
};

export const organizationConsoleManifest = defineConsolePackageManifest(
  consoleSurfaceContract
);
