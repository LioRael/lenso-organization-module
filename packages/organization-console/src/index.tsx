import { defineConsoleModule } from "@lenso/runtime-console-api";

import "./styles.css";
import { organizationConsoleManifest } from "./manifest";
import { OrganizationConsolePage } from "./page";

export const organizationConsoleModule = defineConsoleModule({
  id: organizationConsoleManifest.id,
  surfaces: organizationConsoleManifest.surfaces.map((surface) => ({
    area: surface.area,
    component: OrganizationConsolePage,
    icon: surface.icon,
    label: surface.label,
    navigation: surface.navigation,
    path: surface.route,
  })),
});

export { organizationConsoleManifest } from "./manifest";
export { OrganizationConsolePage } from "./page";
