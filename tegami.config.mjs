export default {
  ignore: ["@lenso/organization-module-workspace"],
  npm: { bumpDep: () => false },
  packages: {
    "lenso-module-organization": {},
    "@lenso/organization-console": {},
  },
};
