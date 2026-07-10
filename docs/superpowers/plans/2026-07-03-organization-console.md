# Organization Console Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Build a first-party Runtime Console package for the `organization` linked module so admins can create organizations, manage members, create/revoke invitations, and configure roles from a real Console workspace.

**Architecture:** Keep organization permissions module-local and expose the UI through `ModuleManifest.console`, matching the auth console pattern. Add only the backend admin actions the Console truly needs, then create `@lenso/organization-console` as a package-backed extension that uses `runtimeConsoleHostApi.adminData` instead of custom Console-only HTTP routes.

**Tech Stack:** Rust 2024, SQLx/Postgres, `platform-module` manifest/admin actions, React 19, TypeScript 6, Vite 8, Tailwind CSS 4, `@lenso/runtime-console-api`, Vitest, GitHub Actions.

---

## File Structure

- Create `/Users/leosouthey/Projects/framework/lenso-organization-module/package.json`: Node workspace scripts for the Console package.
- Create `/Users/leosouthey/Projects/framework/lenso-organization-module/pnpm-workspace.yaml`: package discovery for `packages/*`.
- Create `/Users/leosouthey/Projects/framework/lenso-organization-module/tsconfig.json`: project references for typecheck.
- Modify `/Users/leosouthey/Projects/framework/lenso-organization-module/.github/workflows/ci.yml`: add pnpm install, Console tests, typecheck, and bundle build.
- Modify `/Users/leosouthey/Projects/framework/lenso-organization-module/crates/organization/src/repositories.rs`: add organization archive, custom role creation, role permission update, and role validation helpers.
- Modify `/Users/leosouthey/Projects/framework/lenso-organization-module/crates/organization/src/admin.rs`: add Console-facing admin actions and input validation.
- Modify `/Users/leosouthey/Projects/framework/lenso-organization-module/crates/organization/src/module.rs`: declare Console surfaces in the Rust manifest and expose all new admin actions.
- Modify `/Users/leosouthey/Projects/framework/lenso-organization-module/crates/organization/tests/organization.rs`: add integration tests for the new admin actions and manifest metadata.
- Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/package.json`: npm package metadata for `@lenso/organization-console`.
- Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/vite.config.ts`: bundle config using host-provided React and Runtime Console API.
- Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/console-surface.json`: package contract mirrored by Rust `ModuleManifest.console`.
- Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/runtime-console-theme.css`: fallback theme until the published Runtime Console API package exposes `theme.css`.
- Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/styles.css`: package Tailwind entry.
- Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/manifest.ts`: typed manifest wrapper.
- Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/model.ts`: row normalization, status derivation, filtering, and action input builders.
- Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/page.tsx`: Console UI and action forms.
- Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/index.tsx`: package export for Runtime Console.
- Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/index.test.tsx`: package manifest/export tests.
- Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/model.test.ts`: model tests.
- Modify `/Users/leosouthey/Projects/framework/lenso-organization-module/README.md`: document Console install, actions, and token behavior.
- After `@lenso/organization-console@0.1.0` is published, modify `/Users/leosouthey/Projects/framework/lenso/crates/platform-admin-data/catalogs/lenso-official-module-catalog.json`: advertise the package in the official catalog.

## Product Scope

This plan implements option A:

- A new **Organization** workspace in Runtime Console.
- Sidebar surfaces:
  - `/data/organization` and `/data/organization/organizations` for organizations.
  - `/data/organization/members` for memberships.
  - `/data/organization/roles` for roles.
  - `/data/organization/invitations` for invitations.
- The main Console page is a dense operational UI, not a landing page.
- Console actions:
  - `create_organization`
  - `archive_organization`
  - `create_role`
  - `update_role_permissions`
  - `create_invitation`
  - `revoke_invitation`
  - `update_member_role`
  - `remove_member`
- Invitation creation displays the raw token only in the action response panel. Admin data never exposes `token_hash`.
- Role permissions remain module-local data in `organization.roles.permissions`; they are not injected into `ActorContext.scopes`.
- Email delivery remains out of scope.

### Task 1: Add Console Workspace Scaffolding

**Files:**
- Create: `/Users/leosouthey/Projects/framework/lenso-organization-module/package.json`
- Create: `/Users/leosouthey/Projects/framework/lenso-organization-module/pnpm-workspace.yaml`
- Create: `/Users/leosouthey/Projects/framework/lenso-organization-module/tsconfig.json`
- Create: `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/package.json`
- Create: `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/vite.config.ts`
- Create: `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/runtime-console-theme.css`
- Create: `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/styles.css`

- [x] **Step 1: Create the root Node workspace files**

Create `/Users/leosouthey/Projects/framework/lenso-organization-module/package.json`:

```json
{
  "name": "@lenso/organization-module-workspace",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "pnpm --dir packages/organization-console build",
    "check": "pnpm test && pnpm typecheck && pnpm build",
    "test": "vitest run packages/organization-console/src",
    "typecheck": "tsc -b --pretty false"
  },
  "devDependencies": {
    "@lenso/runtime-console-api": "^0.1.0",
    "@tailwindcss/vite": "^4.3.1",
    "@types/node": "^25.9.2",
    "@types/react": "^19.1.0",
    "react": "^19.1.0",
    "typescript": "^6.0.3",
    "vite": "^8.0.16",
    "vitest": "^4.1.7"
  },
  "packageManager": "pnpm@11.5.0+sha512.dbfcc4f81cf48597afd4bc391ffdf12c11f1a9fb83a395bfa6b0a2d9cc2fd8ffebafdb1ccbd529632153f793904c2615b7f09fe1a345473fd1c35845172a8eb1"
}
```

Create `/Users/leosouthey/Projects/framework/lenso-organization-module/pnpm-workspace.yaml`:

```yaml
packages:
  - "packages/*"
minimumReleaseAgeExclude:
  - '@lenso/runtime-console-api@0.1.0'
```

Create `/Users/leosouthey/Projects/framework/lenso-organization-module/tsconfig.json`:

```json
{
  "files": [],
  "references": [{ "path": "./packages/organization-console" }]
}
```

- [x] **Step 2: Create package metadata and TypeScript config**

Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/package.json`:

```json
{
  "name": "@lenso/organization-console",
  "version": "0.1.0",
  "description": "Runtime Console surface for the Lenso organization module.",
  "license": "MIT",
  "type": "module",
  "repository": {
    "type": "git",
    "url": "git+https://github.com/LioRael/lenso-organization-module.git",
    "directory": "packages/organization-console"
  },
  "homepage": "https://github.com/LioRael/lenso-organization-module#readme",
  "publishConfig": {
    "access": "public"
  },
  "files": [
    "dist",
    "src/index.tsx",
    "src/manifest.ts",
    "src/model.ts",
    "src/page.tsx",
    "console-surface.json",
    "package.json"
  ],
  "scripts": {
    "build": "vite build",
    "prepack": "pnpm build"
  },
  "exports": {
    ".": "./src/index.tsx",
    "./bundle": "./dist/organization-console.js",
    "./style": "./dist/organization-console.css",
    "./console-surface": "./console-surface.json"
  },
  "lenso": {
    "console": {
      "bundle": "./dist/organization-console.js",
      "hostApi": "1",
      "styles": ["./dist/organization-console.css"],
      "surface": "./console-surface.json"
    }
  },
  "peerDependencies": {
    "@lenso/runtime-console-api": "^0.1.0",
    "react": "^19.1.0"
  },
  "peerDependenciesMeta": {
    "@lenso/runtime-console-api": {
      "optional": true
    }
  },
  "devDependencies": {}
}
```

Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/tsconfig.json`:

```json
{
  "compilerOptions": {
    "allowSyntheticDefaultImports": true,
    "composite": true,
    "declaration": true,
    "emitDeclarationOnly": true,
    "jsx": "react-jsx",
    "lib": ["ES2024", "DOM", "DOM.Iterable"],
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "noEmitOnError": true,
    "strict": true,
    "target": "ES2024",
    "types": ["vitest/globals"]
  },
  "include": ["src", "console-surface.json", "vite.config.ts"]
}
```

- [x] **Step 3: Add Vite and style entry files**

Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/vite.config.ts`:

```ts
import { existsSync } from "node:fs";
import { createRequire } from "node:module";
import { resolve } from "node:path";

import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

const hostImports = {
  "@lenso/runtime-console-api":
    "/console/extensions/host/runtime-console-api.js",
  react: "/console/extensions/host/react.js",
  "react/jsx-runtime": "/console/extensions/host/react-jsx-runtime.js",
};
const require = createRequire(import.meta.url);
const siblingRuntimeConsoleApiTheme = resolve(
  import.meta.dirname,
  "../../../lenso-runtime-console/packages/console-package-api/theme.css"
);
const fallbackRuntimeConsoleApiTheme = resolve(
  import.meta.dirname,
  "src/runtime-console-theme.css"
);
const runtimeConsoleApiTheme =
  optionalResolve("@lenso/runtime-console-api/theme.css") ??
  (existsSync(siblingRuntimeConsoleApiTheme)
    ? siblingRuntimeConsoleApiTheme
    : fallbackRuntimeConsoleApiTheme);

function optionalResolve(specifier: string) {
  try {
    return require.resolve(specifier);
  } catch {
    return null;
  }
}

export default defineConfig({
  build: {
    emptyOutDir: true,
    lib: {
      cssFileName: "organization-console",
      entry: resolve(import.meta.dirname, "src/index.tsx"),
      fileName: () => "organization-console.js",
      formats: ["es"],
    },
    rollupOptions: {
      external: Object.keys(hostImports),
      output: {
        paths: hostImports,
      },
    },
  },
  resolve: {
    alias: {
      "@lenso/runtime-console-api/theme.css": runtimeConsoleApiTheme,
    },
  },
  plugins: [tailwindcss()],
});
```

Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/styles.css`:

```css
@reference "@lenso/runtime-console-api/theme.css";

@layer utilities;

@import "tailwindcss/utilities.css" layer(utilities);

@source "./**/*.{ts,tsx}";
```

Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/runtime-console-theme.css` by copying the exported theme CSS fallback from `/Users/leosouthey/Projects/framework/lenso-auth-module/packages/auth-console/src/runtime-console-theme.css`.

- [x] **Step 4: Install package dependencies**

Run:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module
pnpm install
```

Expected: `pnpm-lock.yaml` is created, dependencies install successfully, and no package build runs yet.

- [x] **Step 5: Run the empty workspace checks**

Run:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module
pnpm typecheck
```

Expected: FAIL because `packages/organization-console/src/index.tsx` does not exist yet. This confirms the TS project is wired to the package.

- [x] **Step 6: Commit scaffolding**

Run:

```bash
git -C /Users/leosouthey/Projects/framework/lenso-organization-module add package.json pnpm-workspace.yaml pnpm-lock.yaml tsconfig.json packages/organization-console/package.json packages/organization-console/tsconfig.json packages/organization-console/vite.config.ts packages/organization-console/src/styles.css packages/organization-console/src/runtime-console-theme.css
git -C /Users/leosouthey/Projects/framework/lenso-organization-module commit -m "chore: scaffold organization console package"
```

### Task 2: Add Console-Required Admin Actions

**Files:**
- Modify: `/Users/leosouthey/Projects/framework/lenso-organization-module/crates/organization/src/repositories.rs`
- Modify: `/Users/leosouthey/Projects/framework/lenso-organization-module/crates/organization/src/admin.rs`
- Modify: `/Users/leosouthey/Projects/framework/lenso-organization-module/crates/organization/src/module.rs`
- Modify: `/Users/leosouthey/Projects/framework/lenso-organization-module/crates/organization/tests/organization.rs`

- [x] **Step 1: Write the failing admin action integration test**

Append this test to `/Users/leosouthey/Projects/framework/lenso-organization-module/crates/organization/tests/organization.rs` before `migrated_database()`:

```rust
#[tokio::test]
async fn console_admin_actions_create_archive_and_manage_roles() {
    let Some(db) = migrated_database().await else {
        return;
    };

    seed_user(&db.pool, "usr_console_owner").await;

    let repo = Arc::new(PostgresOrganizationRepository::new(db.pool.clone()));
    let admin = OrganizationAdminData::new(repo.clone());
    let created = admin
        .invoke(
            "create_organization",
            json!({
                "name": "Console Org",
                "slug": "console-org",
                "owner_auth_user_id": "usr_console_owner"
            }),
        )
        .await
        .expect("create organization action");

    let organization_id = created["id"].as_str().expect("organization id");
    assert_eq!(created["name"], "Console Org");
    assert_eq!(created["slug"], "console-org");

    let custom_role = admin
        .invoke(
            "create_role",
            json!({
                "organization_id": organization_id,
                "name": "billing",
                "permissions": [
                    "organization.read",
                    "billing.invoices.read",
                    "billing.invoices.manage"
                ]
            }),
        )
        .await
        .expect("create role action");
    let role_id = custom_role["id"].as_str().expect("role id");
    assert_eq!(custom_role["system_key"], Value::Null);
    assert_eq!(custom_role["permissions"][1], "billing.invoices.read");

    let updated = admin
        .invoke(
            "update_role_permissions",
            json!({
                "role_id": role_id,
                "permissions": ["organization.read", "billing.invoices.read"]
            }),
        )
        .await
        .expect("update role permissions action");
    assert_eq!(updated["updated"], true);

    let role_record = admin
        .get("roles", role_id)
        .await
        .expect("get role")
        .expect("role exists");
    assert_eq!(
        role_record["permissions"],
        json!(["organization.read", "billing.invoices.read"])
    );

    let owner_role = repo
        .owner_role_for_organization(organization_id)
        .await
        .expect("owner role");
    let rejected = admin
        .invoke(
            "update_role_permissions",
            json!({
                "role_id": owner_role.id,
                "permissions": ["organization.read"]
            }),
        )
        .await
        .expect_err("owner role permissions are protected");
    assert!(rejected.to_string().contains("owner role permissions cannot be changed"));

    let archived = admin
        .invoke(
            "archive_organization",
            json!({ "organization_id": organization_id }),
        )
        .await
        .expect("archive organization action");
    assert_eq!(archived["archived"], true);

    let organization_record = admin
        .get("organizations", organization_id)
        .await
        .expect("get organization")
        .expect("organization exists");
    assert!(organization_record["archived_at"].as_str().is_some());

    let visible_to_owner = organization::public::list_user_organizations(
        &db.pool,
        &AuthUserId("usr_console_owner".to_owned()),
    )
    .await
    .expect("list user organizations");
    assert!(visible_to_owner.is_empty());

    db.cleanup().await;
}
```

- [x] **Step 2: Run the failing Rust test**

Run:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module
cargo test --locked -p lenso-module-organization console_admin_actions_create_archive_and_manage_roles
```

Expected: FAIL with an unknown organization admin action error for `create_organization`.

- [x] **Step 3: Add repository methods and permission normalization**

In `/Users/leosouthey/Projects/framework/lenso-organization-module/crates/organization/src/repositories.rs`, add these public methods inside `impl PostgresOrganizationRepository` after `remove_member`:

```rust
    pub async fn create_role(
        &self,
        organization_id: &str,
        name: &str,
        permissions: &[String],
        now: DateTime<Utc>,
    ) -> AppResult<Role> {
        let name = required_trimmed(name, "name")?;
        let permissions = normalize_permissions(permissions)?;
        let organization = self
            .find_organization(organization_id)
            .await?
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, "organization not found"))?;
        if organization.archived_at.is_some() {
            return Err(AppError::new(
                ErrorCode::Validation,
                "organization is archived",
            ));
        }
        let role_id = new_id("org_role");
        sqlx::query_as::<_, RoleRow>(
            r#"
            insert into organization.roles (id, organization_id, name, permissions, system_key, created_at, updated_at)
            values ($1, $2, $3, $4, null, $5, $5)
            returning id, organization_id, name, permissions, system_key, created_at, updated_at
            "#,
        )
        .bind(&role_id)
        .bind(organization_id)
        .bind(name)
        .bind(serde_json::to_value(&permissions).expect("permissions serializes"))
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map(role_from_row)
        .map_err(map_sql_error)
    }

    pub async fn update_role_permissions(
        &self,
        role_id: &str,
        permissions: &[String],
        now: DateTime<Utc>,
    ) -> AppResult<bool> {
        let role = self
            .find_role(role_id)
            .await?
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, "role not found"))?;
        if role.system_key.as_deref() == Some("owner") {
            return Err(AppError::new(
                ErrorCode::Validation,
                "owner role permissions cannot be changed",
            ));
        }
        let permissions = normalize_permissions(permissions)?;
        sqlx::query_scalar::<_, String>(
            r#"
            update organization.roles
            set permissions = $2, updated_at = $3
            where id = $1
            returning id
            "#,
        )
        .bind(role_id)
        .bind(serde_json::to_value(&permissions).expect("permissions serializes"))
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.is_some())
        .map_err(map_sql_error)
    }

    pub async fn archive_organization(
        &self,
        organization_id: &str,
        now: DateTime<Utc>,
    ) -> AppResult<bool> {
        sqlx::query_scalar::<_, String>(
            r#"
            update organization.organizations
            set archived_at = $2, updated_at = $2
            where id = $1 and archived_at is null
            returning id
            "#,
        )
        .bind(organization_id)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.is_some())
        .map_err(map_sql_error)
    }
```

Add this helper near `required_trimmed`:

```rust
fn normalize_permissions(values: &[String]) -> AppResult<Vec<String>> {
    let mut permissions = Vec::new();
    for value in values {
        let value = value.trim();
        if value.is_empty() {
            return Err(AppError::new(
                ErrorCode::Validation,
                "permissions must not contain empty strings",
            ));
        }
        if !permissions.iter().any(|existing| existing == value) {
            permissions.push(value.to_owned());
        }
    }
    if permissions.is_empty() {
        return Err(AppError::new(
            ErrorCode::Validation,
            "permissions is required",
        ));
    }
    Ok(permissions)
}
```

- [x] **Step 4: Add admin action constants, parsing, and dispatch**

In `/Users/leosouthey/Projects/framework/lenso-organization-module/crates/organization/src/admin.rs`, add constants:

```rust
pub const CREATE_ORGANIZATION_ACTION: &str = "create_organization";
pub const ARCHIVE_ORGANIZATION_ACTION: &str = "archive_organization";
pub const CREATE_ROLE_ACTION: &str = "create_role";
pub const UPDATE_ROLE_PERMISSIONS_ACTION: &str = "update_role_permissions";
```

Inside `impl AdminActionSource for OrganizationAdminData`, add these `match` arms before `CREATE_INVITATION_ACTION`:

```rust
            CREATE_ORGANIZATION_ACTION => {
                let name = required_string(&input, "name")?;
                let slug = required_string(&input, "slug")?;
                let owner_auth_user_id = required_string(&input, "owner_auth_user_id")?;
                let organization = self
                    .repository
                    .create_organization_with_owner(
                        name,
                        slug,
                        &auth::public::AuthUserId(owner_auth_user_id.to_owned()),
                        Utc::now(),
                    )
                    .await?;
                Ok(serde_json::json!({
                    "id": organization.id,
                    "name": organization.name,
                    "slug": organization.slug,
                    "created_at": organization.created_at,
                    "updated_at": organization.updated_at,
                    "archived_at": organization.archived_at,
                }))
            }
            ARCHIVE_ORGANIZATION_ACTION => {
                let organization_id = required_string(&input, "organization_id")?;
                let archived = self
                    .repository
                    .archive_organization(organization_id, Utc::now())
                    .await?;
                Ok(serde_json::json!({ "organization_id": organization_id, "archived": archived }))
            }
            CREATE_ROLE_ACTION => {
                let organization_id = required_string(&input, "organization_id")?;
                let name = required_string(&input, "name")?;
                let permissions = required_string_array(&input, "permissions")?;
                let role = self
                    .repository
                    .create_role(organization_id, name, &permissions, Utc::now())
                    .await?;
                Ok(serde_json::json!({
                    "id": role.id,
                    "organization_id": role.organization_id,
                    "name": role.name,
                    "permissions": role.permissions,
                    "system_key": role.system_key,
                    "created_at": role.created_at,
                    "updated_at": role.updated_at,
                }))
            }
            UPDATE_ROLE_PERMISSIONS_ACTION => {
                let role_id = required_string(&input, "role_id")?;
                let permissions = required_string_array(&input, "permissions")?;
                let updated = self
                    .repository
                    .update_role_permissions(role_id, &permissions, Utc::now())
                    .await?;
                Ok(serde_json::json!({ "role_id": role_id, "updated": updated }))
            }
```

Add this parser below `required_string`:

```rust
fn required_string_array(input: &Value, name: &str) -> AppResult<Vec<String>> {
    let values = input
        .get(name)
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::new(ErrorCode::Validation, format!("{name} is required")))?;
    let mut strings = Vec::new();
    for value in values {
        let value = value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::Validation,
                    format!("{name} must contain non-empty strings"),
                )
            })?;
        if !strings.iter().any(|existing| existing == value) {
            strings.push(value.to_owned());
        }
    }
    if strings.is_empty() {
        return Err(AppError::new(
            ErrorCode::Validation,
            format!("{name} is required"),
        ));
    }
    Ok(strings)
}
```

- [x] **Step 5: Expose new actions in the declarative admin surface**

In `/Users/leosouthey/Projects/framework/lenso-organization-module/crates/organization/src/module.rs`, extend the admin import:

```rust
use crate::admin::{
    ARCHIVE_ORGANIZATION_ACTION, CREATE_INVITATION_ACTION, CREATE_ORGANIZATION_ACTION,
    CREATE_ROLE_ACTION, OrganizationAdminData, REMOVE_MEMBER_ACTION, REVOKE_INVITATION_ACTION,
    UPDATE_MEMBER_ROLE_ACTION, UPDATE_ROLE_PERMISSIONS_ACTION,
};
```

In `admin_surface()`, add these actions before `create_invitation_action()`:

```rust
            create_organization_action(),
            string_action(
                ARCHIVE_ORGANIZATION_ACTION,
                "Archive organization",
                ORGANIZATION_MANAGE,
                "organization_id",
                "Organization",
                AdminActionDangerLevel::High,
            ),
            create_role_action(),
            update_role_permissions_action(),
```

Add these helper functions near `create_invitation_action()`:

```rust
fn create_organization_action() -> AdminAction {
    AdminAction {
        name: CREATE_ORGANIZATION_ACTION.to_owned(),
        label: "Create organization".to_owned(),
        capability: ORGANIZATION_MANAGE.to_owned(),
        input_schema: Some(AdminActionInputSchema {
            fields: vec![
                input("name", "Name", FieldType::String, true, None),
                input("slug", "Slug", FieldType::String, true, None),
                input(
                    "owner_auth_user_id",
                    "Owner auth user",
                    FieldType::String,
                    true,
                    None,
                ),
            ],
        }),
        confirmation: None,
        danger_level: AdminActionDangerLevel::Low,
    }
}

fn create_role_action() -> AdminAction {
    AdminAction {
        name: CREATE_ROLE_ACTION.to_owned(),
        label: "Create role".to_owned(),
        capability: ORGANIZATION_ROLES_MANAGE.to_owned(),
        input_schema: Some(AdminActionInputSchema {
            fields: vec![
                input(
                    "organization_id",
                    "Organization",
                    FieldType::String,
                    true,
                    None,
                ),
                input("name", "Name", FieldType::String, true, None),
                input(
                    "permissions",
                    "Permissions",
                    FieldType::Json,
                    true,
                    Some("JSON array of permission strings".to_owned()),
                ),
            ],
        }),
        confirmation: None,
        danger_level: AdminActionDangerLevel::Low,
    }
}

fn update_role_permissions_action() -> AdminAction {
    AdminAction {
        name: UPDATE_ROLE_PERMISSIONS_ACTION.to_owned(),
        label: "Update role permissions".to_owned(),
        capability: ORGANIZATION_ROLES_MANAGE.to_owned(),
        input_schema: Some(AdminActionInputSchema {
            fields: vec![
                input("role_id", "Role", FieldType::String, true, None),
                input(
                    "permissions",
                    "Permissions",
                    FieldType::Json,
                    true,
                    Some("JSON array of permission strings".to_owned()),
                ),
            ],
        }),
        confirmation: None,
        danger_level: AdminActionDangerLevel::Medium,
    }
}
```

- [x] **Step 6: Run the admin action test until it passes**

Run:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module
cargo test --locked -p lenso-module-organization console_admin_actions_create_archive_and_manage_roles
```

Expected: PASS.

- [x] **Step 7: Run the full Rust test suite**

Run:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module
cargo test --locked -p lenso-module-organization
```

Expected: all organization tests pass.

- [x] **Step 8: Commit backend actions**

Run:

```bash
git -C /Users/leosouthey/Projects/framework/lenso-organization-module add crates/organization/src/repositories.rs crates/organization/src/admin.rs crates/organization/src/module.rs crates/organization/tests/organization.rs
git -C /Users/leosouthey/Projects/framework/lenso-organization-module commit -m "feat: add organization console admin actions"
```

### Task 3: Declare Organization Console Surfaces In Rust Manifest

**Files:**
- Modify: `/Users/leosouthey/Projects/framework/lenso-organization-module/crates/organization/src/module.rs`
- Modify: `/Users/leosouthey/Projects/framework/lenso-organization-module/crates/organization/tests/organization.rs`

- [x] **Step 1: Write the failing manifest test**

Add this test to the `#[cfg(test)] mod tests` block in `/Users/leosouthey/Projects/framework/lenso-organization-module/crates/organization/src/module.rs`:

```rust
    #[test]
    fn manifest_declares_console_surfaces() {
        let manifest = manifest();

        assert_eq!(manifest.console, console_surfaces());
        assert_eq!(manifest.console.len(), 4);
        assert_eq!(manifest.console[0].route, "/data/organization");
        assert_eq!(manifest.console[1].route, "/data/organization/members");
        assert_eq!(manifest.console[2].route, "/data/organization/roles");
        assert_eq!(manifest.console[3].route, "/data/organization/invitations");
        assert_eq!(
            manifest.console[0].package.name,
            "@lenso/organization-console"
        );
        assert_eq!(manifest.console[0].package.export, "organizationConsoleModule");
    }
```

- [x] **Step 2: Run the failing manifest test**

Run:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module
cargo test --locked -p lenso-module-organization manifest_declares_console_surfaces
```

Expected: FAIL because `console_surfaces()` is not defined.

- [x] **Step 3: Add Console surface helpers**

In `/Users/leosouthey/Projects/framework/lenso-organization-module/crates/organization/src/module.rs`, extend the `platform_module` import:

```rust
use platform_module::{
    AdminAction, AdminActionDangerLevel, AdminActionInputField, AdminActionInputSchema,
    AdminDeclarativeComponent, AdminDeclarativePage, AdminDeclarativeSection,
    AdminDeclarativeSurface, AdminSchema, ConsoleArea, ConsoleNavigation, ConsolePackage,
    ConsoleSurface, ConsoleWorkspaceRef, EntitySchema, FieldSchema, FieldType, HostLinkedModule,
    LinkedBinding, LinkedHttpContribution, Module, ModuleHttpMethod, ModuleHttpRoute,
    ModuleManifest,
};
```

Add constants near `MODULE_NAME`:

```rust
pub const ORGANIZATION_CONSOLE_PACKAGE: &str = "@lenso/organization-console";
pub const ORGANIZATION_CONSOLE_EXPORT: &str = "organizationConsoleModule";
```

Add these functions before `manifest()`:

```rust
fn organization_workspace() -> ConsoleWorkspaceRef {
    ConsoleWorkspaceRef {
        id: "organization".to_owned(),
        label: "Organization".to_owned(),
        icon: Some("boxes".to_owned()),
    }
}

pub fn console_surfaces() -> Vec<ConsoleSurface> {
    vec![
        console_surface(
            "organizations",
            "Organizations",
            "/data/organization",
            "boxes",
            ORGANIZATION_READ,
            70,
        ),
        console_surface(
            "members",
            "Members",
            "/data/organization/members",
            "users",
            ORGANIZATION_MEMBERS_MANAGE,
            80,
        ),
        console_surface(
            "roles",
            "Roles",
            "/data/organization/roles",
            "shield",
            ORGANIZATION_ROLES_MANAGE,
            90,
        ),
        console_surface(
            "invitations",
            "Invitations",
            "/data/organization/invitations",
            "key-round",
            ORGANIZATION_INVITATIONS_MANAGE,
            100,
        ),
    ]
}

fn console_surface(
    name: &str,
    label: &str,
    route: &str,
    icon: &str,
    capability: &str,
    order: i32,
) -> ConsoleSurface {
    ConsoleSurface {
        name: name.to_owned(),
        label: label.to_owned(),
        area: ConsoleArea::Data,
        route: route.to_owned(),
        package: ConsolePackage {
            name: ORGANIZATION_CONSOLE_PACKAGE.to_owned(),
            export: ORGANIZATION_CONSOLE_EXPORT.to_owned(),
        },
        icon: Some(icon.to_owned()),
        required_capabilities: vec![capability.to_owned()],
        navigation: Some(ConsoleNavigation {
            workspace: organization_workspace(),
            group: None,
            order: Some(order),
        }),
    }
}
```

Update `manifest()`:

```rust
pub fn manifest() -> ModuleManifest {
    ModuleManifest::builder(MODULE_NAME)
        .dependencies(vec![AUTH_MODULE_DEPENDENCY.to_owned()])
        .capabilities(capabilities())
        .http_routes(http_routes())
        .declarative_admin(admin_surface())
        .console(console_surfaces())
        .build()
}
```

- [x] **Step 4: Update the existing manifest assertion**

In `manifest_declares_organization_surfaces()`, add:

```rust
        assert_eq!(manifest.console, console_surfaces());
```

- [x] **Step 5: Run manifest tests**

Run:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module
cargo test --locked -p lenso-module-organization manifest_declares_console_surfaces manifest_declares_organization_surfaces
```

Expected: PASS.

- [x] **Step 6: Commit manifest surfaces**

Run:

```bash
git -C /Users/leosouthey/Projects/framework/lenso-organization-module add crates/organization/src/module.rs
git -C /Users/leosouthey/Projects/framework/lenso-organization-module commit -m "feat: declare organization console surfaces"
```

### Task 4: Add Console Package Contract And Exports

**Files:**
- Create: `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/console-surface.json`
- Create: `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/manifest.ts`
- Create: `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/index.tsx`
- Create: `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/index.test.tsx`
- Create: `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/page.tsx`

- [x] **Step 1: Write the package export test**

Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/index.test.tsx`:

```ts
import { describe, expect, test } from "vitest";

import {
  OrganizationConsolePage,
  organizationConsoleManifest,
  organizationConsoleModule,
} from ".";

describe("organization console package", () => {
  test("declares an installable organization console package export", () => {
    expect(organizationConsoleManifest).toMatchObject({
      exportName: "organizationConsoleModule",
      id: "organization",
      packageName: "@lenso/organization-console",
      source: "runtime_bundle",
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
      version: "workspace",
    });
    expect(organizationConsoleModule).toMatchObject({
      id: "organization",
      surfaces: [
        { label: "Organizations", path: "/data/organization" },
        { label: "Members", path: "/data/organization/members" },
        { label: "Roles", path: "/data/organization/roles" },
        { label: "Invitations", path: "/data/organization/invitations" },
      ],
    });
    expect(OrganizationConsolePage).toBeTypeOf("function");
  });
});
```

- [x] **Step 2: Run the failing package export test**

Run:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module
pnpm test packages/organization-console/src/index.test.tsx
```

Expected: FAIL because `src/index.tsx` does not exist.

- [x] **Step 3: Add the console surface contract**

Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/console-surface.json`:

```json
{
  "exportName": "organizationConsoleModule",
  "id": "organization",
  "bundle": {
    "hostApi": "1",
    "path": "dist/organization-console.js",
    "styles": ["dist/organization-console.css"]
  },
  "packageName": "@lenso/organization-console",
  "source": "runtime_bundle",
  "surfaces": [
    {
      "area": "data",
      "icon": "boxes",
      "label": "Organizations",
      "navigation": {
        "order": 70,
        "workspace": {
          "icon": "boxes",
          "id": "organization",
          "label": "Organization"
        }
      },
      "requiredCapabilities": ["organization.read"],
      "route": "/data/organization",
      "surfaceName": "organizations"
    },
    {
      "area": "data",
      "icon": "users",
      "label": "Members",
      "navigation": {
        "order": 80,
        "workspace": {
          "icon": "boxes",
          "id": "organization",
          "label": "Organization"
        }
      },
      "requiredCapabilities": ["organization.members.manage"],
      "route": "/data/organization/members",
      "surfaceName": "members"
    },
    {
      "area": "data",
      "icon": "shield",
      "label": "Roles",
      "navigation": {
        "order": 90,
        "workspace": {
          "icon": "boxes",
          "id": "organization",
          "label": "Organization"
        }
      },
      "requiredCapabilities": ["organization.roles.manage"],
      "route": "/data/organization/roles",
      "surfaceName": "roles"
    },
    {
      "area": "data",
      "icon": "key-round",
      "label": "Invitations",
      "navigation": {
        "order": 100,
        "workspace": {
          "icon": "boxes",
          "id": "organization",
          "label": "Organization"
        }
      },
      "requiredCapabilities": ["organization.invitations.manage"],
      "route": "/data/organization/invitations",
      "surfaceName": "invitations"
    }
  ],
  "version": "workspace"
}
```

- [x] **Step 4: Add manifest and temporary page exports**

Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/manifest.ts`:

```ts
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
```

Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/page.tsx`:

```tsx
export function OrganizationConsolePage() {
  return (
    <main className="flex h-full flex-col bg-background p-4 text-foreground">
      <header className="border-border border-b pb-3">
        <p className="font-medium text-muted-foreground text-xs uppercase tracking-normal">
          Module console package
        </p>
        <h1 className="font-semibold text-2xl">Organization</h1>
      </header>
    </main>
  );
}
```

Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/index.tsx`:

```tsx
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
```

- [x] **Step 5: Run the package export test**

Run:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module
pnpm test packages/organization-console/src/index.test.tsx
```

Expected: PASS.

- [x] **Step 6: Commit the package contract**

Run:

```bash
git -C /Users/leosouthey/Projects/framework/lenso-organization-module add packages/organization-console/console-surface.json packages/organization-console/src/manifest.ts packages/organization-console/src/page.tsx packages/organization-console/src/index.tsx packages/organization-console/src/index.test.tsx
git -C /Users/leosouthey/Projects/framework/lenso-organization-module commit -m "feat: add organization console package contract"
```

### Task 5: Add Console Data Model

**Files:**
- Create: `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/model.ts`
- Create: `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/model.test.ts`

- [x] **Step 1: Write model tests**

Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/model.test.ts`:

```ts
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
```

- [x] **Step 2: Run the failing model tests**

Run:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module
pnpm test packages/organization-console/src/model.test.ts
```

Expected: FAIL because `src/model.ts` does not exist.

- [x] **Step 3: Implement the model**

Create `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/model.ts`:

```ts
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
    : roles;

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
    : rows;

export const invitationsForOrganization = (
  rows: readonly InvitationRow[],
  organizationId: string | null
): InvitationRow[] =>
  organizationId
    ? rows.filter((row) => row.organizationId === organizationId)
    : rows;

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
```

- [x] **Step 4: Run model tests**

Run:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module
pnpm test packages/organization-console/src/model.test.ts
```

Expected: PASS.

- [x] **Step 5: Commit the model**

Run:

```bash
git -C /Users/leosouthey/Projects/framework/lenso-organization-module add packages/organization-console/src/model.ts packages/organization-console/src/model.test.ts
git -C /Users/leosouthey/Projects/framework/lenso-organization-module commit -m "feat: add organization console model"
```

### Task 6: Build The Organization Console UI

**Files:**
- Modify: `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/page.tsx`
- Modify: `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/model.ts`

- [x] **Step 1: Replace the placeholder page with the operational UI**

Replace `/Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/src/page.tsx` with a dense operational page that:

- Loads four admin-data entities:
  - `{ moduleName: "organization", entityName: "organizations" }`
  - `{ moduleName: "organization", entityName: "memberships" }`
  - `{ moduleName: "organization", entityName: "roles" }`
  - `{ moduleName: "organization", entityName: "invitations" }`
- Uses one selected organization id. Default to the first active organization, then the first row.
- Shows a top summary strip with total orgs, active orgs, archived orgs, active invitations.
- Shows organization rows on the left.
- Shows right-side tabs for `members`, `roles`, and `invitations`.
- Provides forms for all Console actions.

Use this component structure and names:

```tsx
import { runtimeConsoleHostApi } from "@lenso/runtime-console-api";
import { useMemo, useState, type FormEvent } from "react";

import {
  DEFAULT_ROLE_PERMISSIONS,
  buildPermissionInput,
  invitationRows,
  invitationSummary,
  invitationsForOrganization,
  membershipRows,
  membershipsForOrganization,
  organizationRows,
  organizationSummary,
  roleRows,
  rolesForOrganization,
  type InvitationRow,
  type MembershipRow,
  type OrganizationRow,
  type RoleRow,
} from "./model";

type TabId = "members" | "roles" | "invitations";
type AdminActionResponseLike = {
  data?: unknown;
};
type AdminActionMutationOptions = {
  onSuccess?: (response: AdminActionResponseLike) => void;
};
type OrganizationConsoleHostApi = typeof runtimeConsoleHostApi & {
  adminData: Omit<typeof runtimeConsoleHostApi.adminData, "useInvokeAction"> & {
    useInvokeAction: () => {
      error: Error;
      isError: boolean;
      isPending: boolean;
      mutate: (
        request: {
          actionName: string;
          input: Record<string, unknown>;
          moduleName: string;
        },
        options?: AdminActionMutationOptions
      ) => void;
    };
  };
};
const consoleHostApi =
  runtimeConsoleHostApi as unknown as OrganizationConsoleHostApi;

export function OrganizationConsolePage() {
  const organizationsQuery = consoleHostApi.adminData.useRecords({
    entityName: "organizations",
    moduleName: "organization",
  });
  const membershipsQuery = consoleHostApi.adminData.useRecords({
    entityName: "memberships",
    moduleName: "organization",
  });
  const rolesQuery = consoleHostApi.adminData.useRecords({
    entityName: "roles",
    moduleName: "organization",
  });
  const invitationsQuery = consoleHostApi.adminData.useRecords({
    entityName: "invitations",
    moduleName: "organization",
  });
  const action = consoleHostApi.adminData.useInvokeAction();
  const [selectedOrganizationId, setSelectedOrganizationId] = useState<string | null>(null);
  const [tab, setTab] = useState<TabId>("members");
  const [lastToken, setLastToken] = useState<string | null>(null);

  const organizations = organizationRows(organizationsQuery.data?.data ?? []);
  const roles = roleRows(rolesQuery.data?.data ?? []);
  const memberships = membershipRows(membershipsQuery.data?.data ?? []);
  const invitations = invitationRows(invitationsQuery.data?.data ?? []);
  const selectedOrganization =
    organizations.find((organization) => organization.id === selectedOrganizationId) ??
    organizations.find((organization) => organization.status === "active") ??
    organizations[0] ??
    null;
  const selectedId = selectedOrganization?.id ?? null;
  const scopedRoles = rolesForOrganization(roles, selectedId);
  const scopedMemberships = membershipsForOrganization(memberships, selectedId);
  const scopedInvitations = invitationsForOrganization(invitations, selectedId);
  const orgSummary = organizationSummary(organizations);
  const inviteSummary = invitationSummary(scopedInvitations);
  const isLoading =
    organizationsQuery.isPending ||
    membershipsQuery.isPending ||
    rolesQuery.isPending ||
    invitationsQuery.isPending;
  const error =
    organizationsQuery.error ||
    membershipsQuery.error ||
    rolesQuery.error ||
    invitationsQuery.error;

  const invoke = (actionName: string, input: Record<string, unknown>) => {
    action.mutate({ actionName, input, moduleName: "organization" });
  };

  const createOrganization = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    invoke("create_organization", {
      name: requiredFormText(form, "name"),
      owner_auth_user_id: requiredFormText(form, "owner_auth_user_id"),
      slug: requiredFormText(form, "slug"),
    });
    event.currentTarget.reset();
  };

  const archiveOrganization = () => {
    if (selectedId) {
      invoke("archive_organization", { organization_id: selectedId });
    }
  };

  const createRole = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!selectedId) {
      return;
    }
    const form = new FormData(event.currentTarget);
    invoke("create_role", {
      name: requiredFormText(form, "name"),
      organization_id: selectedId,
      permissions: buildPermissionInput(requiredFormText(form, "permissions")),
    });
    event.currentTarget.reset();
  };

  const updateRolePermissions = (roleId: string, permissions: string) => {
    invoke("update_role_permissions", {
      permissions: buildPermissionInput(permissions),
      role_id: roleId,
    });
  };

  const createInvitation = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!selectedId) {
      return;
    }
    const form = new FormData(event.currentTarget);
    setLastToken(null);
    action.mutate(
      {
        actionName: "create_invitation",
        input: {
          email: requiredFormText(form, "email"),
          expires_at: new Date(requiredFormText(form, "expires_at")).toISOString(),
          organization_id: selectedId,
          role_id: requiredFormText(form, "role_id"),
        },
        moduleName: "organization",
      },
      {
        onSuccess: (response) => {
          const data =
            response.data && typeof response.data === "object"
              ? (response.data as { token?: unknown })
              : null;
          const token = data?.token;
          if (typeof token === "string") {
            setLastToken(token);
          }
        },
      }
    );
    event.currentTarget.reset();
  };

  if (error) {
    return <PanelMessage tone="error" value={String(error.message)} />;
  }

  return (
    <main className="flex h-full min-h-0 flex-col bg-background text-foreground">
      <header className="flex flex-wrap items-start gap-3 border-border border-b px-4 py-3">
        <div className="min-w-0">
          <p className="font-medium text-muted-foreground text-xs uppercase tracking-normal">
            Module console package
          </p>
          <h1 className="font-semibold text-2xl">Organization</h1>
        </div>
        <SummaryStrip
          active={orgSummary.active}
          archived={orgSummary.archived}
          invitations={inviteSummary.active}
          total={orgSummary.total}
        />
      </header>

      {isLoading ? (
        <PanelMessage value="Loading organization records" />
      ) : (
        <div className="grid min-h-0 flex-1 grid-cols-[minmax(280px,360px)_minmax(0,1fr)]">
          <OrganizationRail
            onArchive={archiveOrganization}
            onCreate={createOrganization}
            onSelect={setSelectedOrganizationId}
            organizations={organizations}
            selected={selectedOrganization}
          />
          <section className="flex min-w-0 flex-col">
            <TabBar selected={tab} setSelected={setTab} />
            {selectedOrganization ? (
              <div className="min-h-0 flex-1 overflow-auto p-4">
                {tab === "members" ? (
                  <MembersPanel
                    members={scopedMemberships}
                    onRemove={(membershipId) =>
                      invoke("remove_member", { membership_id: membershipId })
                    }
                    onUpdateRole={(membershipId, roleId) =>
                      invoke("update_member_role", {
                        membership_id: membershipId,
                        role_id: roleId,
                      })
                    }
                    roles={scopedRoles}
                  />
                ) : null}
                {tab === "roles" ? (
                  <RolesPanel
                    onCreate={createRole}
                    onUpdate={updateRolePermissions}
                    roles={scopedRoles}
                  />
                ) : null}
                {tab === "invitations" ? (
                  <InvitationsPanel
                    invitations={scopedInvitations}
                    lastToken={lastToken}
                    onCreate={createInvitation}
                    onRevoke={(invitationId) =>
                      invoke("revoke_invitation", { invitation_id: invitationId })
                    }
                    roles={scopedRoles}
                  />
                ) : null}
              </div>
            ) : (
              <PanelMessage value="Create an organization to start managing members and invitations" />
            )}
          </section>
        </div>
      )}
    </main>
  );
}
```

Add the presentational helpers in the same file below `OrganizationConsolePage`. Keep them small and table-based:

```tsx
function requiredFormText(form: FormData, name: string): string {
  return String(form.get(name) ?? "").trim();
}

function SummaryStrip({
  active,
  archived,
  invitations,
  total,
}: {
  active: number;
  archived: number;
  invitations: number;
  total: number;
}) {
  return (
    <div className="ml-auto grid grid-cols-4 border border-border text-xs">
      <Metric label="Total" value={total} />
      <Metric label="Active" value={active} />
      <Metric label="Archived" value={archived} />
      <Metric label="Open invites" value={invitations} />
    </div>
  );
}

function Metric({ label, value }: { label: string; value: number }) {
  return (
    <div className="min-w-22 border-border border-r px-3 py-2 last:border-r-0">
      <div className="font-mono text-foreground text-sm">{value}</div>
      <div className="text-muted-foreground">{label}</div>
    </div>
  );
}

function PanelMessage({
  tone = "muted",
  value,
}: {
  tone?: "error" | "muted";
  value: string;
}) {
  return (
    <div
      className={[
        "m-4 border px-3 py-3 text-sm",
        tone === "error"
          ? "border-(--tone-error-border) bg-(--tone-error-bg) text-(--tone-error-fg)"
          : "border-border text-muted-foreground",
      ].join(" ")}
    >
      {value}
    </div>
  );
}

function OrganizationRail({
  onArchive,
  onCreate,
  onSelect,
  organizations,
  selected,
}: {
  onArchive: () => void;
  onCreate: (event: FormEvent<HTMLFormElement>) => void;
  onSelect: (organizationId: string) => void;
  organizations: OrganizationRow[];
  selected: OrganizationRow | null;
}) {
  return (
    <aside className="flex min-h-0 flex-col border-border border-r">
      <form className="grid gap-2 border-border border-b p-3" onSubmit={onCreate}>
        <div className="font-medium text-foreground text-sm">Create organization</div>
        <input
          className="border border-border bg-background px-2 py-1 text-sm"
          name="name"
          placeholder="Name"
          required
        />
        <input
          className="border border-border bg-background px-2 py-1 font-mono text-sm"
          name="slug"
          placeholder="slug"
          required
        />
        <input
          className="border border-border bg-background px-2 py-1 font-mono text-sm"
          name="owner_auth_user_id"
          placeholder="owner auth user id"
          required
        />
        <button className="border border-border px-2 py-1 text-xs" type="submit">
          Create
        </button>
      </form>
      <div className="min-h-0 flex-1 overflow-auto">
        {organizations.map((organization) => (
          <button
            aria-pressed={selected?.id === organization.id}
            className={[
              "grid w-full gap-1 border-border border-b px-3 py-2 text-left",
              selected?.id === organization.id
                ? "native-selection"
                : "hover:bg-(--bg-row-hover)",
            ].join(" ")}
            key={organization.id}
            onClick={() => onSelect(organization.id)}
            type="button"
          >
            <span className="truncate font-medium text-sm">{organization.name}</span>
            <span className="truncate font-mono text-muted-foreground text-xs">
              {organization.slug} · {organization.id}
            </span>
            <StatusPill status={organization.status} />
          </button>
        ))}
      </div>
      {selected ? (
        <div className="border-border border-t p-3">
          <button
            className="w-full border border-border px-2 py-1 text-xs"
            disabled={selected.status === "archived"}
            onClick={onArchive}
            type="button"
          >
            Archive selected
          </button>
        </div>
      ) : null}
    </aside>
  );
}

function TabBar({
  selected,
  setSelected,
}: {
  selected: TabId;
  setSelected: (tab: TabId) => void;
}) {
  const tabs: { id: TabId; label: string }[] = [
    { id: "members", label: "Members" },
    { id: "roles", label: "Roles" },
    { id: "invitations", label: "Invitations" },
  ];
  return (
    <div className="flex border-border border-b">
      {tabs.map((tab) => (
        <button
          aria-pressed={selected === tab.id}
          className={[
            "border-border border-r px-3 py-2 text-sm",
            selected === tab.id ? "bg-(--bg-row-selected) text-(--fg-on-accent)" : "",
          ].join(" ")}
          key={tab.id}
          onClick={() => setSelected(tab.id)}
          type="button"
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}

function MembersPanel({
  members,
  onRemove,
  onUpdateRole,
  roles,
}: {
  members: MembershipRow[];
  onRemove: (membershipId: string) => void;
  onUpdateRole: (membershipId: string, roleId: string) => void;
  roles: RoleRow[];
}) {
  return (
    <section className="border border-border">
      <PanelHeader label="Members" value={`${members.length} records`} />
      <div className="overflow-x-auto">
        <table className="w-full min-w-220 text-left text-sm">
          <thead className="border-border border-b text-muted-foreground">
            <tr>
              <th className="px-3 py-2 font-medium">Auth user</th>
              <th className="px-3 py-2 font-medium">Role</th>
              <th className="px-3 py-2 font-medium">Created</th>
              <th className="px-3 py-2 font-medium">Status</th>
              <th className="px-3 py-2 font-medium">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border">
            {members.map((member) => (
              <tr key={member.id}>
                <td className="px-3 py-2 font-mono text-xs">{member.authUserId}</td>
                <td className="px-3 py-2">{member.roleName}</td>
                <td className="px-3 py-2 font-mono text-muted-foreground text-xs">
                  {member.createdAt}
                </td>
                <td className="px-3 py-2">
                  <StatusPill status={member.status} />
                </td>
                <td className="px-3 py-2">
                  <form
                    className="flex flex-wrap gap-2"
                    onSubmit={(event) => {
                      event.preventDefault();
                      const form = new FormData(event.currentTarget);
                      onUpdateRole(member.id, requiredFormText(form, "role_id"));
                    }}
                  >
                    <select
                      className="border border-border bg-background px-2 py-1 text-xs"
                      defaultValue={member.roleId}
                      disabled={member.status === "removed"}
                      name="role_id"
                    >
                      {roles.map((role) => (
                        <option key={role.id} value={role.id}>
                          {role.name}
                        </option>
                      ))}
                    </select>
                    <button
                      className="border border-border px-2 py-1 text-xs"
                      disabled={member.status === "removed"}
                      type="submit"
                    >
                      Update
                    </button>
                    <button
                      className="border border-border px-2 py-1 text-xs"
                      disabled={member.status === "removed"}
                      onClick={() => onRemove(member.id)}
                      type="button"
                    >
                      Remove
                    </button>
                  </form>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function RolesPanel({
  onCreate,
  onUpdate,
  roles,
}: {
  onCreate: (event: FormEvent<HTMLFormElement>) => void;
  onUpdate: (roleId: string, permissions: string) => void;
  roles: RoleRow[];
}) {
  return (
    <section className="grid gap-4">
      <form className="grid gap-2 border border-border p-3" onSubmit={onCreate}>
        <div className="font-medium text-sm">Create role</div>
        <input
          className="border border-border bg-background px-2 py-1 text-sm"
          name="name"
          placeholder="Role name"
          required
        />
        <textarea
          className="min-h-24 border border-border bg-background px-2 py-1 font-mono text-xs"
          defaultValue={DEFAULT_ROLE_PERMISSIONS.join("\n")}
          name="permissions"
          required
        />
        <button className="w-fit border border-border px-2 py-1 text-xs" type="submit">
          Create role
        </button>
      </form>
      <div className="grid gap-3">
        {roles.map((role) => (
          <form
            className="grid gap-2 border border-border p-3"
            key={role.id}
            onSubmit={(event) => {
              event.preventDefault();
              const form = new FormData(event.currentTarget);
              onUpdate(role.id, requiredFormText(form, "permissions"));
            }}
          >
            <div className="flex flex-wrap gap-2">
              <span className="font-medium text-sm">{role.name}</span>
              <StatusPill status={role.systemLabel} />
              <span className="ml-auto text-muted-foreground text-xs">
                {role.permissionCount} permissions
              </span>
            </div>
            <textarea
              className="min-h-24 border border-border bg-background px-2 py-1 font-mono text-xs"
              defaultValue={role.permissions.join("\n")}
              disabled={!role.editable}
              name="permissions"
            />
            <button
              className="w-fit border border-border px-2 py-1 text-xs"
              disabled={!role.editable}
              type="submit"
            >
              Update permissions
            </button>
          </form>
        ))}
      </div>
    </section>
  );
}

function InvitationsPanel({
  invitations,
  lastToken,
  onCreate,
  onRevoke,
  roles,
}: {
  invitations: InvitationRow[];
  lastToken: string | null;
  onCreate: (event: FormEvent<HTMLFormElement>) => void;
  onRevoke: (invitationId: string) => void;
  roles: RoleRow[];
}) {
  const defaultExpiry = new Date(Date.now() + 24 * 60 * 60 * 1000)
    .toISOString()
    .slice(0, 16);
  return (
    <section className="grid gap-4">
      <form className="grid gap-2 border border-border p-3" onSubmit={onCreate}>
        <div className="font-medium text-sm">Create invitation</div>
        <input
          className="border border-border bg-background px-2 py-1 text-sm"
          name="email"
          placeholder="member@example.com"
          required
          type="email"
        />
        <select className="border border-border bg-background px-2 py-1 text-sm" name="role_id">
          {roles.map((role) => (
            <option key={role.id} value={role.id}>
              {role.name}
            </option>
          ))}
        </select>
        <input
          className="border border-border bg-background px-2 py-1 font-mono text-sm"
          defaultValue={defaultExpiry}
          name="expires_at"
          required
          type="datetime-local"
        />
        <button className="w-fit border border-border px-2 py-1 text-xs" type="submit">
          Create invitation
        </button>
      </form>
      {lastToken ? (
        <div className="border border-(--tone-warning-border) bg-(--tone-warning-bg) p-3">
          <div className="font-medium text-sm text-(--tone-warning-fg)">
            Invitation token returned once
          </div>
          <div className="mt-2 break-all font-mono text-xs">{lastToken}</div>
        </div>
      ) : null}
      <div className="border border-border">
        <PanelHeader label="Invitations" value={`${invitations.length} records`} />
        <div className="overflow-x-auto">
          <table className="w-full min-w-220 text-left text-sm">
            <thead className="border-border border-b text-muted-foreground">
              <tr>
                <th className="px-3 py-2 font-medium">Email</th>
                <th className="px-3 py-2 font-medium">Role</th>
                <th className="px-3 py-2 font-medium">Expires</th>
                <th className="px-3 py-2 font-medium">Status</th>
                <th className="px-3 py-2 font-medium">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {invitations.map((invitation) => (
                <tr key={invitation.id}>
                  <td className="px-3 py-2">{invitation.email}</td>
                  <td className="px-3 py-2 font-mono text-xs">{invitation.roleId}</td>
                  <td className="px-3 py-2 font-mono text-muted-foreground text-xs">
                    {invitation.expiresAt}
                  </td>
                  <td className="px-3 py-2">
                    <StatusPill status={invitation.status} />
                  </td>
                  <td className="px-3 py-2">
                    <button
                      className="border border-border px-2 py-1 text-xs"
                      disabled={invitation.status !== "active"}
                      onClick={() => onRevoke(invitation.id)}
                      type="button"
                    >
                      Revoke
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </section>
  );
}

function PanelHeader({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center gap-3 border-border border-b px-3 py-2">
      <h2 className="font-medium text-sm">{label}</h2>
      <span className="ml-auto text-muted-foreground text-xs">{value}</span>
    </div>
  );
}

function StatusPill({ status }: { status: string }) {
  return (
    <span className="w-fit border border-border px-2 py-0.5 text-muted-foreground text-xs">
      {status}
    </span>
  );
}
```

- [x] **Step 2: Keep the mutation response typing local to the package**

Do not change Runtime Console core only to support token display. Keep this package-local type in `page.tsx` so the UI can read `response.data.token` from the existing TanStack mutation response:

```ts
type AdminActionResponseLike = {
  data?: unknown;
};
type AdminActionMutationOptions = {
  onSuccess?: (response: AdminActionResponseLike) => void;
};
```

Use this callback for invitation creation:

```ts
onSuccess: (response: AdminActionResponseLike) => {
  const data =
    response.data && typeof response.data === "object"
      ? (response.data as { token?: unknown })
      : null;
  const token = data?.token;
  if (typeof token === "string") {
    setLastToken(token);
  }
},
```

- [x] **Step 3: Run typecheck**

Run:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module
pnpm typecheck
```

Expected: PASS. If the host API mutation type does not support per-call callbacks, change `createInvitation` to use a second `useState` message driven by `action.data` only if the public API exposes mutation data. Keep token display tied to the actual `create_invitation` response.

- [x] **Step 4: Run package tests**

Run:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module
pnpm test
```

Expected: PASS.

- [x] **Step 5: Run package build**

Run:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module
pnpm build
```

Expected: `packages/organization-console/dist/organization-console.js` and `organization-console.css` are created.

- [x] **Step 6: Commit Console UI**

Run:

```bash
git -C /Users/leosouthey/Projects/framework/lenso-organization-module add packages/organization-console/src/page.tsx packages/organization-console/src/model.ts
git -C /Users/leosouthey/Projects/framework/lenso-organization-module commit -m "feat: build organization console UI"
```

### Task 7: Wire CI And Documentation

**Files:**
- Modify: `/Users/leosouthey/Projects/framework/lenso-organization-module/.github/workflows/ci.yml`
- Modify: `/Users/leosouthey/Projects/framework/lenso-organization-module/README.md`

- [x] **Step 1: Update CI**

Replace `/Users/leosouthey/Projects/framework/lenso-organization-module/.github/workflows/ci.yml` with:

```yaml
name: CI

on:
  pull_request:
  push:
    branches:
      - main

jobs:
  check:
    name: Check
    runs-on: ubuntu-latest
    steps:
      - name: Checkout organization module
        uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt

      - name: Setup pnpm
        uses: pnpm/action-setup@v4
        with:
          version: 11.5.0

      - name: Setup Node
        uses: actions/setup-node@v5
        with:
          node-version: 26
          cache: pnpm

      - name: Install Console dependencies
        run: pnpm install --frozen-lockfile

      - name: Check Rust
        run: |
          cargo fmt --all --check
          cargo test --locked -p lenso-module-organization

      - name: Check Console
        run: |
          pnpm test
          pnpm typecheck
          pnpm build
```

- [x] **Step 2: Update README usage**

Add this section to `/Users/leosouthey/Projects/framework/lenso-organization-module/README.md` after "What It Provides":

```md
## Runtime Console

The module declares package-backed Console surfaces through
`organization::module::linked_module()`.

Install the Rust module first:

```sh
lenso module install auth
lenso module install organization
```

Install the Console extension package when your host uses Runtime Console
extensions:

```sh
pnpm add @lenso/organization-console
```

The Console workspace exposes:

- Organizations: create and archive organizations.
- Members: inspect memberships, update member roles, and remove members.
- Roles: create custom roles and update role permission arrays. The seeded
  `owner` role is protected from permission edits.
- Invitations: create invitations, copy the raw token returned once, and revoke
  unaccepted invitations.

Invitation tokens are still stored only as hashes in Postgres. The Console never
lists `token_hash`, and this module still does not send email.
```

- [x] **Step 3: Run the full local checks**

Run:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module
cargo fmt --all --check
cargo test --locked -p lenso-module-organization
pnpm check
```

Expected: all commands pass.

- [x] **Step 4: Commit CI and docs**

Run:

```bash
git -C /Users/leosouthey/Projects/framework/lenso-organization-module add .github/workflows/ci.yml README.md
git -C /Users/leosouthey/Projects/framework/lenso-organization-module commit -m "docs: document organization console"
```

### Task 8: Verify In A Generated Host

**Files:**
- No committed files unless this task exposes a defect. Use `/tmp/lenso-organization-console-proof` for the generated host.

- [x] **Step 1: Create a fresh host and install modules**

Run:

```bash
rm -rf /tmp/lenso-organization-console-proof
cd /tmp
lenso new lenso-organization-console-proof
cd /tmp/lenso-organization-console-proof
lenso module install auth
lenso module install organization
```

Expected:

- Host `Cargo.toml` contains `lenso-module-auth` and `lenso-module-organization`.
- Host composition includes `.linked_module(organization::module::linked_module())`.

- [x] **Step 2: Confirm manifest metadata from Rust**

Run:

```bash
cd /tmp/lenso-organization-console-proof
cargo check
```

Expected: PASS.

- [x] **Step 3: Build the Console extension bundle**

Run:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module
pnpm build
```

Expected: `packages/organization-console/dist/organization-console.js` exists.

- [x] **Step 4: Copy the local extension into the generated host**

Run:

```bash
mkdir -p /tmp/lenso-organization-console-proof/.lenso/console/extensions/organization
cp /Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/dist/organization-console.js /tmp/lenso-organization-console-proof/.lenso/console/extensions/organization/organization-console.js
cp /Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console/dist/organization-console.css /tmp/lenso-organization-console-proof/.lenso/console/extensions/organization/organization-console.css
cat > /tmp/lenso-organization-console-proof/.lenso/console/extensions/registry.json <<'JSON'
{
  "bundles": [
    {
      "module": "organization",
      "packageName": "@lenso/organization-console",
      "exportName": "organizationConsoleModule",
      "entry": "/console/extensions/organization/organization-console.js",
      "styles": ["/console/extensions/organization/organization-console.css"]
    }
  ]
}
JSON
```

Expected: registry file points at the local bundle and style file.

- [x] **Step 5: Run the host and inspect Console metadata**

Run the host with a database using the same local Postgres proof flow used for the module release. Then query:

```bash
curl -sS -H 'authorization: Bearer dev-user:usr_console:console.admin,organization.read,organization.members.manage,organization.roles.manage,organization.invitations.manage' \
  http://127.0.0.1:3000/admin/data/modules | jq '.data[] | select(.module_name == "organization") | {module_name, console}'
```

Expected: the organization module is loaded and includes four Console surfaces with package `@lenso/organization-console`.

- [x] **Step 6: HTTP/static/action smoke**

Fetch:

```text
http://127.0.0.1:3000/console/data/organization
```

Expected:

- `/console/data/organization` returns the hosted Runtime Console HTML.
- Runtime Console assets and organization extension assets return 200.
- `/admin/data/modules` exposes the four organization Console surfaces.
- Creating an organization through admin actions creates seeded owner/admin/member roles.
- Creating an invitation returns a raw token once.
- Invitation admin data never exposes `token_hash`.

Note: Playwright/browser visual automation was not run because this workspace did not have Playwright installed. The generated-host smoke used HTTP, static asset, and admin action checks.

- [x] **Step 7: Record verification in the PR body**

Add this exact checklist to the PR body:

```md
## Verification

- [x] `cargo fmt --all --check`
- [x] `cargo test --locked -p lenso-module-organization`
- [x] `pnpm check`
- [x] Generated host `cargo check`
- [x] `/admin/data/modules` shows organization Console surfaces
- [x] HTTP/static/action smoke for `/console/data/organization`
```

### Task 9: Add Official Catalog Entry After npm Publish

Status: deferred until `@lenso/organization-console@0.1.0` is published. The README documents local smoke testing without claiming the package is already available through the official catalog.

**Files:**
- Modify: `/Users/leosouthey/Projects/framework/lenso/crates/platform-admin-data/catalogs/lenso-official-module-catalog.json`

- [ ] **Step 1: Publish or pack the Console package**

For local validation before npm publish, run:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module/packages/organization-console
pnpm pack
```

Expected: `lenso-organization-console-0.1.0.tgz` is created.

For the public catalog path, publish through the Trusted Publisher workflow:

```bash
cd /Users/leosouthey/Projects/framework/lenso-organization-module
gh workflow run release.yml -f publish_npm=true --ref main
```

Expected: `@lenso/organization-console@0.1.0` exists on npm.

- [ ] **Step 2: Update the official module catalog**

In `/Users/leosouthey/Projects/framework/lenso/crates/platform-admin-data/catalogs/lenso-official-module-catalog.json`, add a `consolePackages` item for `organization`:

```json
{
  "module": "organization",
  "packageName": "@lenso/organization-console",
  "version": "0.1.0",
  "bundleUrl": "https://cdn.jsdelivr.net/npm/@lenso/organization-console@0.1.0/dist/organization-console.js",
  "entry": "/console/extensions/organization/organization-console.js",
  "styleUrls": [
    "https://cdn.jsdelivr.net/npm/@lenso/organization-console@0.1.0/dist/organization-console.css"
  ]
}
```

Preserve the existing catalog shape. If the catalog already has an `organization` entry, add the package to that entry instead of creating a duplicate module record.

- [ ] **Step 3: Run catalog tests**

Run:

```bash
cd /Users/leosouthey/Projects/framework/lenso
cargo test --locked -p platform-admin-data official
```

Expected: PASS. If no test name includes `official`, run:

```bash
cd /Users/leosouthey/Projects/framework/lenso
cargo test --locked -p platform-admin-data
```

Expected: PASS.

- [ ] **Step 4: Commit catalog update**

Run:

```bash
git -C /Users/leosouthey/Projects/framework/lenso add crates/platform-admin-data/catalogs/lenso-official-module-catalog.json
git -C /Users/leosouthey/Projects/framework/lenso commit -m "feat: add organization console to official catalog"
```

## Final Verification

Run these in `/Users/leosouthey/Projects/framework/lenso-organization-module`:

```bash
cargo fmt --all --check
cargo test --locked -p lenso-module-organization
pnpm check
```

Run this in `/Users/leosouthey/Projects/framework/lenso` only after the catalog task:

```bash
cargo test --locked -p platform-admin-data
```

Manual host proof:

```bash
lenso module install auth
lenso module install organization
```

Confirm:

- host `Cargo.toml` includes `lenso-module-organization`
- host composition includes `.linked_module(organization::module::linked_module())`
- `/admin/data/modules` includes organization Console surfaces
- `/console/data/organization` renders the package UI when the extension registry includes the bundle

## Self-Review

- Spec coverage: the plan covers backend admin actions, Rust manifest Console metadata, package-backed React UI, package build/test wiring, generated-host smoke, docs, CI, and official catalog follow-through.
- Placeholder scan: the plan contains exact file paths, commands, action names, package names, routes, and concrete expected outcomes.
- Type consistency: Rust action names match TypeScript invocations and package surface metadata. The package export is consistently `organizationConsoleModule`; the npm package is consistently `@lenso/organization-console`.
