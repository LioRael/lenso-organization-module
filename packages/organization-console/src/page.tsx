import { runtimeConsoleHostApi } from "@lenso/runtime-console-api";
import { useState, type FormEvent } from "react";

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
  type OrganizationSummary,
  type RoleRow,
} from "./model";

type AdminActionResponseLike = { data?: unknown };
type AdminActionMutationOptions = {
  onSuccess?: (response: AdminActionResponseLike) => void;
};
type OrganizationAction = {
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
type OrganizationConsoleHostApi = typeof runtimeConsoleHostApi & {
  adminData: Omit<typeof runtimeConsoleHostApi.adminData, "useInvokeAction"> & {
    useInvokeAction: () => OrganizationAction;
  };
};

type TabId = "members" | "roles" | "invitations";

const consoleHostApi =
  runtimeConsoleHostApi as unknown as OrganizationConsoleHostApi;

const tabs = [
  { id: "members", label: "Members" },
  { id: "roles", label: "Roles" },
  { id: "invitations", label: "Invitations" },
] as const satisfies readonly { id: TabId; label: string }[];

const inputClass =
  "h-8 w-full border border-border bg-background px-2 text-foreground text-sm outline-none native-selection focus:border-foreground disabled:cursor-not-allowed disabled:opacity-60";
const textareaClass =
  "min-h-24 w-full resize-y border border-border bg-background px-2 py-2 font-mono text-foreground text-xs outline-none native-selection focus:border-foreground disabled:cursor-not-allowed disabled:opacity-60";
const buttonClass =
  "h-8 border border-border bg-background px-3 font-medium text-foreground text-xs hover:bg-(--bg-row-hover) disabled:cursor-not-allowed disabled:opacity-50";
const dangerButtonClass =
  "h-8 border border-border bg-background px-3 font-medium text-foreground text-xs hover:bg-(--bg-row-hover) disabled:cursor-not-allowed disabled:opacity-50";
const labelClass = "space-y-1 text-muted-foreground text-xs";

export function OrganizationConsolePage() {
  const [selectedOrganizationId, setSelectedOrganizationId] = useState<
    string | null
  >(null);
  const [activeTab, setActiveTab] = useState<TabId>("members");
  const [lastToken, setLastToken] = useState<string | null>(null);

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
  const action = consoleHostApi.adminData.useInvokeAction() as OrganizationAction;

  const organizations = organizationRows(organizationsQuery.data?.data ?? []);
  const memberships = membershipRows(membershipsQuery.data?.data ?? []);
  const roles = roleRows(rolesQuery.data?.data ?? []);
  const invitations = invitationRows(invitationsQuery.data?.data ?? []);
  const orgSummary = organizationSummary(organizations);

  const displayedOrganizationId = resolveSelectedOrganizationId(
    selectedOrganizationId,
    organizations
  );
  const selectedOrganization =
    organizations.find((org) => org.id === displayedOrganizationId) ?? null;
  const selectedMemberships = membershipsForOrganization(
    memberships,
    displayedOrganizationId
  );
  const selectedRoles = rolesForOrganization(roles, displayedOrganizationId);
  const selectedInvitations = invitationsForOrganization(
    invitations,
    displayedOrganizationId
  );
  const selectedInvitationSummary = invitationSummary(selectedInvitations);
  const isLoading =
    organizationsQuery.isPending ||
    organizationsQuery.isLoading ||
    membershipsQuery.isPending ||
    membershipsQuery.isLoading ||
    rolesQuery.isPending ||
    rolesQuery.isLoading ||
    invitationsQuery.isPending ||
    invitationsQuery.isLoading;
  const queryErrors = [
    organizationsQuery.isError
      ? `Organizations: ${organizationsQuery.error.message}`
      : null,
    membershipsQuery.isError
      ? `Members: ${membershipsQuery.error.message}`
      : null,
    rolesQuery.isError ? `Roles: ${rolesQuery.error.message}` : null,
    invitationsQuery.isError
      ? `Invitations: ${invitationsQuery.error.message}`
      : null,
  ].filter((message): message is string => message !== null);

  const invokeOrganizationAction = (
    actionName: string,
    input: Record<string, unknown>,
    options?: AdminActionMutationOptions
  ) => {
    action.mutate({ actionName, input, moduleName: "organization" }, options);
  };

  const handleCreateOrganization = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const formElement = event.currentTarget;
    const form = new FormData(formElement);
    invokeOrganizationAction("create_organization", {
      name: requiredFormText(form, "name"),
      owner_auth_user_id: requiredFormText(form, "owner_auth_user_id"),
      slug: requiredFormText(form, "slug"),
    });
    formElement.reset();
  };

  const handleArchiveSelected = () => {
    if (displayedOrganizationId === null) {
      return;
    }
    invokeOrganizationAction("archive_organization", {
      organization_id: displayedOrganizationId,
    });
  };

  const handleCreateRole = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (displayedOrganizationId === null) {
      return;
    }
    const formElement = event.currentTarget;
    const form = new FormData(formElement);
    invokeOrganizationAction("create_role", {
      name: requiredFormText(form, "name"),
      organization_id: displayedOrganizationId,
      permissions: buildPermissionInput(requiredFormText(form, "permissions")),
    });
    formElement.reset();
  };

  const handleUpdateRolePermissions = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    invokeOrganizationAction("update_role_permissions", {
      permissions: buildPermissionInput(requiredFormText(form, "permissions")),
      role_id: requiredFormText(form, "role_id"),
    });
  };

  const handleSelectOrganization = (id: string) => {
    setSelectedOrganizationId(id);
    setLastToken(null);
  };

  const handleCreateInvitation = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (displayedOrganizationId === null) {
      return;
    }
    setLastToken(null);
    const formElement = event.currentTarget;
    const form = new FormData(formElement);
    invokeOrganizationAction(
      "create_invitation",
      {
        email: requiredFormText(form, "email"),
        expires_at: new Date(
          requiredFormText(form, "expires_at")
        ).toISOString(),
        organization_id: displayedOrganizationId,
        role_id: requiredFormText(form, "role_id"),
      },
      {
        onSuccess: (response) => {
          setLastToken(invitationTokenFromResponse(response.data));
        },
      }
    );
    formElement.reset();
  };

  const handleRevokeInvitation = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    invokeOrganizationAction("revoke_invitation", {
      invitation_id: requiredFormText(form, "invitation_id"),
    });
  };

  const handleUpdateMemberRole = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    invokeOrganizationAction("update_member_role", {
      membership_id: requiredFormText(form, "membership_id"),
      role_id: requiredFormText(form, "role_id"),
    });
  };

  const handleRemoveMember = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    invokeOrganizationAction("remove_member", {
      membership_id: requiredFormText(form, "membership_id"),
    });
  };

  return (
    <main className="flex h-full flex-col gap-4 overflow-auto bg-background p-4 text-foreground">
      <header className="flex flex-wrap items-start gap-3 border-border border-b pb-3">
        <div className="min-w-0">
          <p className="font-medium text-muted-foreground text-xs uppercase tracking-normal">
            Module console package
          </p>
          <h1 className="font-semibold text-2xl">Organization</h1>
          {selectedOrganization ? (
            <p className="mt-1 truncate font-mono text-muted-foreground text-xs">
              {selectedOrganization.id}
            </p>
          ) : null}
        </div>
        <div className="ml-auto flex flex-wrap gap-2 text-xs">
          <span className="border border-border px-2 py-1 text-muted-foreground">
            linked module
          </span>
          <span className="border border-border px-2 py-1 text-muted-foreground">
            admin-data
          </span>
        </div>
      </header>

      <SummaryStrip
        activeInvitations={selectedInvitationSummary.active}
        organizationSummary={orgSummary}
        selectedOrganization={selectedOrganization}
      />

      {action.isError ? (
        <PanelMessage
          tone="error"
          title="Action failed"
          value={action.error.message}
        />
      ) : null}

      <section className="grid min-h-0 flex-1 gap-4 xl:grid-cols-[360px_minmax(0,1fr)]">
        <OrganizationRail
          action={action}
          onArchiveSelected={handleArchiveSelected}
          onCreateOrganization={handleCreateOrganization}
          onSelect={handleSelectOrganization}
          organizations={organizations}
          selectedOrganization={selectedOrganization}
          selectedOrganizationId={displayedOrganizationId}
        />

        <div className="min-w-0 border border-border bg-card">
          {queryErrors.length > 0 ? (
            <PanelMessage
              tone="error"
              title="Failed to load organization data"
              value={queryErrors.join(" / ")}
            />
          ) : isLoading ? (
            <PanelMessage
              title="Loading organization data"
              value="Reading organizations, members, roles, and invitations."
            />
          ) : displayedOrganizationId === null ? (
            <PanelMessage value="Create an organization to start managing members and invitations" />
          ) : (
            <>
              <div className="flex flex-wrap items-center gap-3 border-border border-b px-3 py-2">
                <PanelHeader
                  detail={selectedOrganization?.slug ?? "-"}
                  title={selectedOrganization?.name ?? "Organization"}
                />
                <TabBar activeTab={activeTab} onChange={setActiveTab} />
              </div>

              {activeTab === "members" ? (
                <MembersPanel
                  action={action}
                  members={selectedMemberships}
                  onRemoveMember={handleRemoveMember}
                  onUpdateMemberRole={handleUpdateMemberRole}
                  roles={selectedRoles}
                />
              ) : activeTab === "roles" ? (
                <RolesPanel
                  action={action}
                  onCreateRole={handleCreateRole}
                  onUpdateRolePermissions={handleUpdateRolePermissions}
                  roles={selectedRoles}
                />
              ) : (
                <InvitationsPanel
                  action={action}
                  invitations={selectedInvitations}
                  lastToken={lastToken}
                  onCreateInvitation={handleCreateInvitation}
                  onRevokeInvitation={handleRevokeInvitation}
                  roles={selectedRoles}
                />
              )}
            </>
          )}
        </div>
      </section>
    </main>
  );
}

function requiredFormText(form: FormData, name: string): string {
  const value = form.get(name);
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`${name} is required`);
  }
  return value.trim();
}

function SummaryStrip({
  activeInvitations,
  organizationSummary,
  selectedOrganization,
}: {
  activeInvitations: number;
  organizationSummary: OrganizationSummary;
  selectedOrganization: OrganizationRow | null;
}) {
  return (
    <section className="grid gap-2 md:grid-cols-4">
      <Metric label="Total orgs" value={organizationSummary.total} />
      <Metric label="Active orgs" value={organizationSummary.active} />
      <Metric label="Archived orgs" value={organizationSummary.archived} />
      <Metric
        label="Active invites"
        value={selectedOrganization ? activeInvitations : "-"}
      />
    </section>
  );
}

function Metric({ label, value }: { label: string; value: number | string }) {
  return (
    <div className="border border-border bg-card px-3 py-2">
      <div className="text-muted-foreground text-xs">{label}</div>
      <div className="mt-1 font-semibold text-2xl text-foreground">
        {value}
      </div>
    </div>
  );
}

function PanelMessage({
  title,
  tone = "neutral",
  value,
}: {
  title?: string;
  tone?: "error" | "neutral" | "warning";
  value: string;
}) {
  const toneClass =
    tone === "error"
      ? "border-red-500/40 text-red-700 dark:text-red-300"
      : tone === "warning"
        ? "border-yellow-500/40 text-foreground"
        : "border-border text-muted-foreground";
  return (
    <div className={`m-3 border px-3 py-3 text-sm ${toneClass}`}>
      {title ? (
        <div className="mb-1 font-medium text-foreground">{title}</div>
      ) : null}
      <div>{value}</div>
    </div>
  );
}

function OrganizationRail({
  action,
  onArchiveSelected,
  onCreateOrganization,
  onSelect,
  organizations,
  selectedOrganization,
  selectedOrganizationId,
}: {
  action: OrganizationAction;
  onArchiveSelected: () => void;
  onCreateOrganization: (event: FormEvent<HTMLFormElement>) => void;
  onSelect: (id: string) => void;
  organizations: OrganizationRow[];
  selectedOrganization: OrganizationRow | null;
  selectedOrganizationId: string | null;
}) {
  return (
    <aside className="min-w-0 border border-border bg-card">
      <div className="border-border border-b px-3 py-2">
        <PanelHeader detail={`${organizations.length} records`} title="Organizations" />
      </div>
      <form
        className="grid gap-2 border-border border-b p-3"
        onSubmit={onCreateOrganization}
      >
        <label className={labelClass}>
          Name
          <input className={inputClass} name="name" required />
        </label>
        <label className={labelClass}>
          Slug
          <input className={inputClass} name="slug" required />
        </label>
        <label className={labelClass}>
          Owner auth user
          <input className={inputClass} name="owner_auth_user_id" required />
        </label>
        <button
          className={buttonClass}
          disabled={action.isPending}
          type="submit"
        >
          Create organization
        </button>
      </form>
      <div className="max-h-[420px] overflow-auto">
        {organizations.length === 0 ? (
          <p className="px-3 py-3 text-muted-foreground text-sm">
            No organizations found.
          </p>
        ) : (
          <ul className="divide-y divide-border">
            {organizations.map((organization) => {
              const selected = organization.id === selectedOrganizationId;
              return (
                <li key={organization.id}>
                  <button
                    className={`block w-full px-3 py-2 text-left hover:bg-(--bg-row-hover) ${
                      selected ? "bg-(--bg-row-hover)" : ""
                    }`}
                    onClick={() => onSelect(organization.id)}
                    type="button"
                  >
                    <div className="flex items-center gap-2">
                      <span className="min-w-0 truncate font-medium text-foreground text-sm">
                        {organization.name}
                      </span>
                      <StatusPill status={organization.status} />
                    </div>
                    <div className="mt-1 truncate font-mono text-muted-foreground text-xs">
                      {organization.slug} / {organization.id}
                    </div>
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </div>
      <div className="border-border border-t p-3">
        <button
          className={dangerButtonClass}
          disabled={
            action.isPending ||
            selectedOrganizationId === null ||
            selectedOrganization?.status === "archived"
          }
          onClick={onArchiveSelected}
          type="button"
        >
          Archive selected
        </button>
      </div>
    </aside>
  );
}

function TabBar({
  activeTab,
  onChange,
}: {
  activeTab: TabId;
  onChange: (tab: TabId) => void;
}) {
  return (
    <div className="ml-auto flex border border-border">
      {tabs.map((tab) => (
        <button
          className={`h-8 px-3 font-medium text-xs ${
            activeTab === tab.id
              ? "bg-foreground text-background"
              : "text-muted-foreground hover:bg-(--bg-row-hover)"
          }`}
          key={tab.id}
          onClick={() => onChange(tab.id)}
          type="button"
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}

function MembersPanel({
  action,
  members,
  onRemoveMember,
  onUpdateMemberRole,
  roles,
}: {
  action: OrganizationAction;
  members: MembershipRow[];
  onRemoveMember: (event: FormEvent<HTMLFormElement>) => void;
  onUpdateMemberRole: (event: FormEvent<HTMLFormElement>) => void;
  roles: RoleRow[];
}) {
  if (members.length === 0) {
    return <PanelMessage value="No members found for this organization." />;
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full min-w-[760px] text-left text-sm">
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
          {members.map((member) => {
            const removed = member.status === "removed";
            const hasCurrentRole = roles.some((role) => role.id === member.roleId);
            return (
              <tr className="hover:bg-(--bg-row-hover)" key={member.id}>
                <td className="px-3 py-2">
                  <div className="font-mono text-foreground text-xs">
                    {member.authUserId}
                  </div>
                  <div className="mt-1 font-mono text-muted-foreground text-xs">
                    {member.id}
                  </div>
                </td>
                <td className="px-3 py-2 text-muted-foreground">
                  {member.roleName}
                </td>
                <td className="px-3 py-2 font-mono text-muted-foreground text-xs">
                  {member.createdAt}
                </td>
                <td className="px-3 py-2">
                  <StatusPill status={member.status} />
                </td>
                <td className="px-3 py-2">
                  <div className="flex flex-wrap gap-2">
                    <form
                      className="flex gap-2"
                      onSubmit={onUpdateMemberRole}
                    >
                      <input
                        name="membership_id"
                        type="hidden"
                        value={member.id}
                      />
                      <select
                        className={inputClass}
                        defaultValue={member.roleId}
                        disabled={removed || roles.length === 0}
                        name="role_id"
                        required
                      >
                        {!hasCurrentRole ? (
                          <option value={member.roleId}>
                            {member.roleName}
                          </option>
                        ) : null}
                        {roles.map((role) => (
                          <option key={role.id} value={role.id}>
                            {role.name}
                          </option>
                        ))}
                      </select>
                      <button
                        className={buttonClass}
                        disabled={action.isPending || removed || roles.length === 0}
                        type="submit"
                      >
                        Update
                      </button>
                    </form>
                    <form onSubmit={onRemoveMember}>
                      <input
                        name="membership_id"
                        type="hidden"
                        value={member.id}
                      />
                      <button
                        className={dangerButtonClass}
                        disabled={action.isPending || removed}
                        type="submit"
                      >
                        Remove
                      </button>
                    </form>
                  </div>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function RolesPanel({
  action,
  onCreateRole,
  onUpdateRolePermissions,
  roles,
}: {
  action: OrganizationAction;
  onCreateRole: (event: FormEvent<HTMLFormElement>) => void;
  onUpdateRolePermissions: (event: FormEvent<HTMLFormElement>) => void;
  roles: RoleRow[];
}) {
  return (
    <div>
      <form
        className="grid gap-2 border-border border-b p-3 md:grid-cols-[220px_minmax(0,1fr)_auto]"
        onSubmit={onCreateRole}
      >
        <label className={labelClass}>
          Role name
          <input className={inputClass} name="name" required />
        </label>
        <label className={labelClass}>
          Permissions
          <textarea
            className={textareaClass}
            defaultValue={DEFAULT_ROLE_PERMISSIONS.join("\n")}
            name="permissions"
            required
          />
        </label>
        <button
          className={`${buttonClass} self-end`}
          disabled={action.isPending}
          type="submit"
        >
          Create role
        </button>
      </form>

      {roles.length === 0 ? (
        <PanelMessage value="No roles found for this organization." />
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full min-w-[820px] text-left text-sm">
            <thead className="border-border border-b text-muted-foreground">
              <tr>
                <th className="px-3 py-2 font-medium">Role</th>
                <th className="px-3 py-2 font-medium">System</th>
                <th className="px-3 py-2 font-medium">Permissions</th>
                <th className="px-3 py-2 font-medium">Updated</th>
                <th className="px-3 py-2 font-medium">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {roles.map((role) => (
                <tr className="align-top hover:bg-(--bg-row-hover)" key={role.id}>
                  <td className="px-3 py-2">
                    <div className="font-medium text-foreground">
                      {role.name}
                    </div>
                    <div className="mt-1 font-mono text-muted-foreground text-xs">
                      {role.id}
                    </div>
                  </td>
                  <td className="px-3 py-2">
                    <StatusPill status={role.systemLabel} />
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">
                    {role.permissionCount} permissions
                  </td>
                  <td className="px-3 py-2 font-mono text-muted-foreground text-xs">
                    {role.updatedAt}
                  </td>
                  <td className="px-3 py-2">
                    <form
                      className="grid min-w-[320px] gap-2"
                      onSubmit={onUpdateRolePermissions}
                    >
                      <input name="role_id" type="hidden" value={role.id} />
                      <textarea
                        className={textareaClass}
                        defaultValue={role.permissions.join("\n")}
                        disabled={!role.editable}
                        name="permissions"
                        required
                      />
                      <button
                        className={buttonClass}
                        disabled={action.isPending || !role.editable}
                        type="submit"
                      >
                        Save permissions
                      </button>
                    </form>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function InvitationsPanel({
  action,
  invitations,
  lastToken,
  onCreateInvitation,
  onRevokeInvitation,
  roles,
}: {
  action: OrganizationAction;
  invitations: InvitationRow[];
  lastToken: string | null;
  onCreateInvitation: (event: FormEvent<HTMLFormElement>) => void;
  onRevokeInvitation: (event: FormEvent<HTMLFormElement>) => void;
  roles: RoleRow[];
}) {
  return (
    <div>
      <form
        className="grid gap-2 border-border border-b p-3 md:grid-cols-[minmax(180px,1fr)_minmax(180px,1fr)_minmax(180px,1fr)_auto]"
        onSubmit={onCreateInvitation}
      >
        <label className={labelClass}>
          Email
          <input className={inputClass} name="email" required type="email" />
        </label>
        <label className={labelClass}>
          Role
          <select
            className={inputClass}
            disabled={roles.length === 0}
            name="role_id"
            required
          >
            {roles.map((role) => (
              <option key={role.id} value={role.id}>
                {role.name}
              </option>
            ))}
          </select>
        </label>
        <label className={labelClass}>
          Expires
          <input
            className={inputClass}
            defaultValue={defaultInvitationExpiry()}
            name="expires_at"
            required
            type="datetime-local"
          />
        </label>
        <button
          className={`${buttonClass} self-end`}
          disabled={action.isPending || roles.length === 0}
          type="submit"
        >
          Create invitation
        </button>
      </form>

      {lastToken ? (
        <PanelMessage
          tone="warning"
          title="Invitation token"
          value={lastToken}
        />
      ) : null}

      {invitations.length === 0 ? (
        <PanelMessage value="No invitations found for this organization." />
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full min-w-[820px] text-left text-sm">
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
                <tr
                  className="hover:bg-(--bg-row-hover)"
                  key={invitation.id}
                >
                  <td className="px-3 py-2">
                    <div className="text-foreground">{invitation.email}</div>
                    <div className="mt-1 font-mono text-muted-foreground text-xs">
                      {invitation.id}
                    </div>
                  </td>
                  <td className="px-3 py-2 font-mono text-muted-foreground text-xs">
                    {invitation.roleId}
                  </td>
                  <td className="px-3 py-2 font-mono text-muted-foreground text-xs">
                    {invitation.expiresAt}
                  </td>
                  <td className="px-3 py-2">
                    <StatusPill status={invitation.status} />
                  </td>
                  <td className="px-3 py-2">
                    <form onSubmit={onRevokeInvitation}>
                      <input
                        name="invitation_id"
                        type="hidden"
                        value={invitation.id}
                      />
                      <button
                        className={dangerButtonClass}
                        disabled={
                          action.isPending || invitation.status !== "active"
                        }
                        type="submit"
                      >
                        Revoke
                      </button>
                    </form>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function PanelHeader({ detail, title }: { detail?: string; title: string }) {
  return (
    <div className="min-w-0">
      <h2 className="truncate font-medium text-foreground text-sm">{title}</h2>
      {detail ? (
        <p className="mt-0.5 truncate text-muted-foreground text-xs">
          {detail}
        </p>
      ) : null}
    </div>
  );
}

function StatusPill({ status }: { status: string }) {
  return (
    <span className="inline-flex h-6 items-center border border-border px-2 font-medium text-muted-foreground text-xs">
      {status}
    </span>
  );
}

function resolveSelectedOrganizationId(
  selectedOrganizationId: string | null,
  organizations: readonly OrganizationRow[]
): string | null {
  if (
    selectedOrganizationId !== null &&
    organizations.some((organization) => organization.id === selectedOrganizationId)
  ) {
    return selectedOrganizationId;
  }
  return (
    organizations.find((organization) => organization.status === "active")?.id ??
    organizations[0]?.id ??
    null
  );
}

function invitationTokenFromResponse(data: unknown): string | null {
  return isRecord(data) && typeof data.token === "string" ? data.token : null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function defaultInvitationExpiry(): string {
  const date = new Date();
  date.setDate(date.getDate() + 7);
  const offsetMs = date.getTimezoneOffset() * 60_000;
  return new Date(date.getTime() - offsetMs).toISOString().slice(0, 16);
}
