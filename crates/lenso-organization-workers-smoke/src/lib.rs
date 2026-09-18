//! Private real-Workers qualification Host, including typed calls and App removal.
#![cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use lenso_app_plan::{
    AppComposition, CapabilityBinding, CapabilityEndpointPlan, CapabilityRequirementPlan,
    PluginInstancePlan,
};
use lenso_capability_organization_admin as admin;
use lenso_capability_organization_directory as directory;
use lenso_capability_organization_membership as membership;
use lenso_capability_organization_membership_admin as members;
use lenso_kernel::{Kernel, RuntimeFailure, ShutdownOutcome};
use lenso_native_adapter::{
    NativePluginFactory, NativePluginFactoryContext, NativePluginInstance, NativePluginRegistry,
};
use lenso_organization_d1_plugin::{self as plugin, workers::D1Binding};
use lenso_workers_driver::WorkersDriver;
use serde_json::json;
use std::time::Duration;
use wasm_bindgen::prelude::*;

#[derive(Debug)]
struct Caller;
impl NativePluginFactory for Caller {
    fn package_id(&self) -> &'static str {
        "test.organization-caller"
    }
    fn instantiate(
        &self,
        _: NativePluginFactoryContext<'_>,
    ) -> Result<NativePluginInstance, RuntimeFailure> {
        Ok(NativePluginInstance::default())
    }
}
struct Event(WorkersDriver);
impl Drop for Event {
    fn drop(&mut self) {
        self.0.request_shutdown();
    }
}
fn failure() -> JsValue {
    JsValue::from_str("Organization qualification failed")
}

#[wasm_bindgen]
pub async fn migrate(batch: js_sys::Function) -> Result<(), JsValue> {
    plugin::schema::plan()
        .map_err(|_| failure())?
        .setup(&D1Binding(batch))
        .await
        .map_err(|_| failure())
}

fn composition(config: String) -> Result<lenso_app_plan::ResolvedAppPlan, JsValue> {
    let mut callers = vec![
        PluginInstancePlan::new("caller", "test.organization-caller"),
        PluginInstancePlan::new("caller:other", "test.organization-caller"),
    ];
    let mut provider =
        PluginInstancePlan::new("organization", plugin::PACKAGE_ID).with_configuration(config);
    let mut bindings = vec![];
    for (cap, version, mut ops) in [
        (
            admin::CAPABILITY_ID,
            admin::DESCRIPTOR_VERSION,
            vec![
                admin::CREATE_ORGANIZATION_OPERATION,
                admin::LIST_ORGANIZATIONS_OPERATION,
            ],
        ),
        (
            members::CAPABILITY_ID,
            members::DESCRIPTOR_VERSION,
            vec![
                members::ADD_MEMBER_OPERATION,
                members::REMOVE_MEMBER_OPERATION,
                members::LIST_MEMBERS_OPERATION,
            ],
        ),
        (
            membership::CAPABILITY_ID,
            membership::DESCRIPTOR_VERSION,
            vec![membership::CHECK_MEMBERSHIP_OPERATION],
        ),
        (
            directory::CAPABILITY_ID,
            directory::DESCRIPTOR_VERSION,
            vec![
                directory::GET_ORGANIZATION_OPERATION,
                directory::LIST_FOR_SUBJECT_OPERATION,
            ],
        ),
    ] {
        ops.sort_unstable();
        for caller in &mut callers {
            *caller = caller
                .clone()
                .with_requirement(CapabilityRequirementPlan::one(cap, version));
        }
        for caller in ["caller", "caller:other"] {
            bindings.push(CapabilityBinding::new(caller, cap, version, "organization"));
        }
        provider = provider.with_capability(CapabilityEndpointPlan::new(cap, version, ops));
    }
    callers.push(provider);
    AppComposition::new(callers, bindings)
        .resolve()
        .map_err(|_| failure())
}

#[wasm_bindgen]
pub async fn exercise(
    batch: js_sys::Function,
    mode: String,
    id: String,
    close: js_sys::Function,
) -> Result<String, JsValue> {
    let config = json!({"binding":"ORGANIZATION","admin_callers":["caller"],"membership_admin_callers":["caller"],"directory_callers":["caller"]});
    let driver = WorkersDriver::new();
    let _event = Event(driver.clone());
    let started = Kernel::start_native(
        composition(config.to_string())?,
        driver.clone(),
        NativePluginRegistry::new()
            .with_factory(Caller)
            .with_factory(plugin::workers::factory(
                if mode == "wrong-binding" {
                    "WRONG"
                } else {
                    "ORGANIZATION"
                },
                batch,
            )),
    )
    .await;
    if matches!(
        mode.as_str(),
        "missing-schema" | "wrong-binding" | "throws" | "malformed"
    ) {
        return if started.is_err() {
            Ok("startup-rejected".into())
        } else {
            Err(failure())
        };
    }
    let app = started.map_err(|_| failure())?;
    if mode == "closed" {
        close.call0(&JsValue::UNDEFINED).map_err(|_| failure())?;
    }
    let request = admin::CreateOrganizationRequest {
        idempotency_key: id.clone(),
        slug: id.clone(),
        name: "  Organization  ".into(),
        owner_subject: "owner".into(),
    };
    let created = app
        .invoke::<admin::OrganizationAdminCreateOrganization>(
            if mode == "forbidden" {
                "caller:other"
            } else {
                "caller"
            },
            admin::CREATE_ORGANIZATION_OPERATION,
            request.clone(),
        )
        .await;
    let mut outcome = "passed".to_owned();
    if matches!(mode.as_str(), "closed" | "rollback") {
        if !matches!(created, Err(RuntimeFailure::PluginFailure { .. })) {
            return Err(failure());
        }
    } else if mode == "forbidden" {
        if created.map_err(|_| failure())? != Err(admin::CreateOrganizationError::Forbidden) {
            return Err(failure());
        }
        let add = app
            .invoke::<members::OrganizationMembershipAdminAddMember>(
                "caller:other",
                members::ADD_MEMBER_OPERATION,
                members::AddMemberRequest {
                    organization_id: "absent".into(),
                    idempotency_key: id.clone(),
                    subject: "member".into(),
                },
            )
            .await
            .map_err(|_| failure())?;
        if add != Err(members::AddMemberError::Forbidden) {
            return Err(failure());
        }
        let remove = app
            .invoke::<members::OrganizationMembershipAdminRemoveMember>(
                "caller:other",
                members::REMOVE_MEMBER_OPERATION,
                members::RemoveMemberRequest {
                    organization_id: "absent".into(),
                    idempotency_key: id,
                    subject: "member".into(),
                },
            )
            .await
            .map_err(|_| failure())?;
        if remove != Err(members::RemoveMemberError::Forbidden) {
            return Err(failure());
        }
    } else {
        let created = created.map_err(|_| failure())?.map_err(|_| failure())?;
        let replay = app
            .invoke::<admin::OrganizationAdminCreateOrganization>(
                "caller",
                admin::CREATE_ORGANIZATION_OPERATION,
                request,
            )
            .await
            .map_err(|_| failure())?
            .map_err(|_| failure())?;
        if replay.created
            || replay.organization_id != created.organization_id
            || replay.owner_membership_id != created.owner_membership_id
        {
            return Err(failure());
        }
        let org = created.organization_id;
        let add = app
            .invoke::<members::OrganizationMembershipAdminAddMember>(
                "caller",
                members::ADD_MEMBER_OPERATION,
                members::AddMemberRequest {
                    organization_id: org.clone(),
                    idempotency_key: format!("add-{id}"),
                    subject: "member".into(),
                },
            )
            .await
            .map_err(|_| failure())?
            .map_err(|_| failure())?;
        if add.revision != "1" {
            return Err(failure());
        }
        if mode == "race-add" {
            outcome=json!({"created":add.created,"membership_id":add.membership_id,"organization_id":org}).to_string();
        } else {
            let owner = app
                .invoke::<membership::OrganizationMembership>(
                    "caller:other",
                    membership::CHECK_MEMBERSHIP_OPERATION,
                    membership::CheckMembershipRequest {
                        organization_id: org.clone(),
                        subject: "owner".into(),
                    },
                )
                .await
                .map_err(|_| failure())?
                .map_err(|_| failure())?;
            if !owner.active || !owner.owner {
                return Err(failure());
            }
            let protected = app
                .invoke::<members::OrganizationMembershipAdminRemoveMember>(
                    "caller",
                    members::REMOVE_MEMBER_OPERATION,
                    members::RemoveMemberRequest {
                        organization_id: org.clone(),
                        idempotency_key: format!("owner-{id}"),
                        subject: "owner".into(),
                    },
                )
                .await
                .map_err(|_| failure())?;
            if protected != Err(members::RemoveMemberError::OwnerProtected) {
                return Err(failure());
            }
            let removed = app
                .invoke::<members::OrganizationMembershipAdminRemoveMember>(
                    "caller",
                    members::REMOVE_MEMBER_OPERATION,
                    members::RemoveMemberRequest {
                        organization_id: org.clone(),
                        idempotency_key: format!("remove-{id}"),
                        subject: "member".into(),
                    },
                )
                .await
                .map_err(|_| failure())?
                .map_err(|_| failure())?;
            if removed.revision != "2" {
                return Err(failure());
            }
            let page = app
                .invoke::<members::OrganizationMembershipAdminListMembers>(
                    "caller",
                    members::LIST_MEMBERS_OPERATION,
                    members::ListMembersRequest {
                        organization_id: org.clone(),
                        cursor: None,
                        limit: 1,
                        status: members::ListMembersRequestStatus::Active,
                        subject: None,
                    },
                )
                .await
                .map_err(|_| failure())?
                .map_err(|_| failure())?;
            if page.members.len() != 1 || !page.members[0].is_owner || page.next_cursor.is_some() {
                return Err(failure());
            }
            let entry = app
                .invoke::<directory::OrganizationDirectoryGetOrganization>(
                    "caller",
                    directory::GET_ORGANIZATION_OPERATION,
                    directory::GetOrganizationRequest {
                        organization_id: org,
                    },
                )
                .await
                .map_err(|_| failure())?
                .map_err(|_| failure())?;
            if entry.name != "Organization" || entry.revision != "1" {
                return Err(failure());
            }
            let page = app
                .invoke::<admin::OrganizationAdminListOrganizations>(
                    "caller",
                    admin::LIST_ORGANIZATIONS_OPERATION,
                    admin::ListOrganizationsRequest {
                        cursor: None,
                        limit: 1,
                        slug: Some(id),
                        status: admin::ListOrganizationsRequestStatus::Active,
                    },
                )
                .await
                .map_err(|_| failure())?
                .map_err(|_| failure())?;
            if page.organizations.len() != 1 || page.next_cursor.is_some() {
                return Err(failure());
            }
        }
    }
    if app.shutdown(Duration::from_secs(1)).await != ShutdownOutcome::Clean {
        return Err(failure());
    }
    let remaining = AppComposition::new(
        vec![PluginInstancePlan::new(
            "caller",
            "test.organization-caller",
        )],
        vec![],
    )
    .resolve()
    .map_err(|_| failure())?;
    let app = Kernel::start_native(
        remaining,
        driver,
        NativePluginRegistry::new().with_factory(Caller),
    )
    .await
    .map_err(|_| failure())?;
    if app.shutdown(Duration::from_secs(1)).await != ShutdownOutcome::Clean {
        return Err(failure());
    }
    Ok(outcome)
}
