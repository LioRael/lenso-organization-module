import { defineConsoleModule } from "@lenso/runtime-console-api";

// @ts-expect-error Vite handles CSS side-effect imports for console packages.
import "./styles.css";
import { organizationConsoleManifest } from "./manifest.js";
import { OrganizationConsolePage } from "./page.js";

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

export { organizationConsoleManifest } from "./manifest.js";
export { OrganizationConsolePage } from "./page.js";
