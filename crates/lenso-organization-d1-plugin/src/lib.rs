//! Organization on primary D1, with exact caller Instance admission.
//! Calling Plugins retain final resource authorization and RBAC responsibility.
pub mod schema;
mod storage;
#[cfg(test)]
mod tests;
#[cfg(target_arch = "wasm32")]
pub mod workers;

use futures::future::LocalBoxFuture;
use lenso_capability_organization_admin as admin;
use lenso_capability_organization_directory as directory;
use lenso_capability_organization_membership as membership;
use lenso_capability_organization_membership_admin as members;
use lenso_kernel::{
    DeactivateContext, InvocationContext, NativeRequestEndpoint, NativeRequestFuture, PluginFuture,
    PluginLifecycle, PrepareContext, RuntimeFailure,
};
use lenso_migration_d1::{Error, Statement, Transport};
use lenso_native_adapter::{NativePluginFactory, NativePluginFactoryContext, NativePluginInstance};
use lenso_organization_core::{
    exact_caller, valid_membership_request, valid_name, valid_organization_name, valid_slug,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{cell::Cell, fmt, rc::Rc};

pub const PACKAGE_ID: &str = "lenso.organization.d1";
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct D1Config {
    pub binding: String,
    pub admin_callers: Vec<String>,
    #[serde(default)]
    pub directory_callers: Vec<String>,
    #[serde(default)]
    pub membership_admin_callers: Vec<String>,
}
impl D1Config {
    fn validate(&self) -> Result<(), RuntimeFailure> {
        if self.binding.is_empty()
            || self.binding.len() > 128
            || !self.binding.as_bytes()[0].is_ascii_alphabetic()
            || !self
                .binding
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || self.admin_callers.is_empty()
            || self
                .admin_callers
                .iter()
                .chain(&self.directory_callers)
                .chain(&self.membership_admin_callers)
                .any(|v| !valid_name(v, 256))
        {
            return Err(RuntimeFailure::InvalidResolvedPlan {
                detail: "invalid Organization D1 binding or exact Instance allowlists".into(),
            });
        }
        Ok(())
    }
}
/// Event-owned primary binding. Implementations must execute each batch atomically.
pub trait Binding: Transport + fmt::Debug {}
impl<T: Transport + fmt::Debug> Binding for T {}

#[derive(Clone, Debug)]
struct EventBinding(Rc<dyn Binding>);
impl Transport for EventBinding {
    fn batch(
        &self,
        statements: Vec<Statement>,
    ) -> LocalBoxFuture<'_, Result<Vec<Vec<Value>>, Error>> {
        self.0.batch(statements)
    }
}
#[derive(Clone, Debug)]
struct OrganizationFactory {
    name: String,
    binding: EventBinding,
}
/// Create a fresh factory for each Workers event; match the configured binding exactly.
pub fn factory(name: impl Into<String>, binding: Rc<dyn Binding>) -> impl NativePluginFactory {
    OrganizationFactory {
        name: name.into(),
        binding: EventBinding(binding),
    }
}
impl NativePluginFactory for OrganizationFactory {
    fn package_id(&self) -> &'static str {
        PACKAGE_ID
    }
    fn package_version(&self) -> &'static str {
        PACKAGE_VERSION
    }
    fn instantiate(
        &self,
        context: NativePluginFactoryContext<'_>,
    ) -> Result<NativePluginInstance, RuntimeFailure> {
        let config: D1Config = serde_json::from_str(context.configuration()).map_err(|_| {
            RuntimeFailure::InvalidResolvedPlan {
                detail: "invalid Organization D1 configuration".into(),
            }
        })?;
        config.validate()?;
        if context.entrypoint() != "default" || config.binding != self.name {
            return Err(RuntimeFailure::InvalidResolvedPlan {
                detail:
                    "Organization D1 requires its exact configured binding and default entrypoint"
                        .into(),
            });
        }
        let provider = Provider {
            config,
            binding: self.binding.clone(),
            prepared: Rc::new(Cell::new(false)),
        };
        let endpoints: Vec<Rc<dyn NativeRequestEndpoint>> = vec![
            Rc::new(admin::OrganizationAdminEndpoint::new(provider.clone())),
            Rc::new(directory::OrganizationDirectoryEndpoint::new(
                provider.clone(),
            )),
            Rc::new(membership::OrganizationMembershipEndpoint::new(
                provider.clone(),
            )),
            Rc::new(members::OrganizationMembershipAdminEndpoint::new(
                provider.clone(),
            )),
        ];
        Ok(NativePluginInstance::with_lifecycle(endpoints, provider))
    }
}
#[derive(Clone, Debug)]
struct Provider {
    config: D1Config,
    binding: EventBinding,
    prepared: Rc<Cell<bool>>,
}
impl Provider {
    fn ready(&self) -> Result<(), RuntimeFailure> {
        if self.prepared.get() {
            Ok(())
        } else {
            Err(failure(&Error::Transport))
        }
    }
}
impl PluginLifecycle for Provider {
    fn prepare(&self, _: PrepareContext) -> PluginFuture {
        let provider = self.clone();
        Box::pin(async move {
            provider.prepared.set(false);
            schema::plan()
                .map_err(|error| failure(&error))?
                .verify(&provider.binding)
                .await
                .map_err(|error| failure(&error))?;
            provider.prepared.set(true);
            Ok(())
        })
    }
    fn deactivate(&self, _: DeactivateContext) -> PluginFuture {
        self.prepared.set(false);
        Box::pin(async { Ok(()) })
    }
}
fn failure(error: &Error) -> RuntimeFailure {
    RuntimeFailure::PluginFailure {
        detail: error.to_string(),
    }
}
fn valid_page(limit: i64, cursor: Option<&str>) -> bool {
    (1..=100).contains(&limit) && cursor.is_none_or(|v| valid_name(v, 256))
}

impl admin::OrganizationAdminProvider for Provider {
    fn create_organization(
        &self,
        context: InvocationContext,
        request: admin::CreateOrganizationRequest,
    ) -> NativeRequestFuture<admin::OrganizationAdminCreateOrganization> {
        let provider = self.clone();
        Box::pin(async move {
            provider.ready()?;
            let invalid = (!valid_name(&request.idempotency_key, 256)
                || !valid_organization_name(&request.name)
                || !valid_slug(&request.slug)
                || !valid_name(&request.owner_subject, 256))
            .then_some("invalid_organization");
            storage::mutate(
                &provider.binding,
                storage::Mutation::Create,
                context.caller_instance(),
                &provider.config.admin_callers,
                &request,
                invalid,
            )
            .await
            .map_err(|error| failure(&error))
        })
    }
    fn list_organizations(
        &self,
        context: InvocationContext,
        request: admin::ListOrganizationsRequest,
    ) -> NativeRequestFuture<admin::OrganizationAdminListOrganizations> {
        let provider = self.clone();
        Box::pin(async move {
            if exact_caller(context.caller_instance(), &provider.config.admin_callers).is_none() {
                return Ok(Err(admin::ListOrganizationsError::Forbidden));
            }
            if !valid_page(request.limit, request.cursor.as_deref()) {
                return Ok(Err(admin::ListOrganizationsError::InvalidPage));
            }
            if !request.slug.as_deref().is_none_or(valid_slug) {
                return Ok(Err(admin::ListOrganizationsError::InvalidRequest));
            }
            provider.ready()?;
            storage::read(&provider.binding, storage::LIST_ORGANIZATIONS, &request)
                .await
                .map_err(|error| failure(&error))
        })
    }
}

impl members::OrganizationMembershipAdminProvider for Provider {
    fn add_member(
        &self,
        context: InvocationContext,
        request: members::AddMemberRequest,
    ) -> NativeRequestFuture<members::OrganizationMembershipAdminAddMember> {
        let provider = self.clone();
        Box::pin(async move {
            provider.ready()?;
            let invalid = (!valid_membership_request(
                &request.idempotency_key,
                &request.organization_id,
                &request.subject,
            ))
            .then_some("invalid_request");
            storage::mutate(
                &provider.binding,
                storage::Mutation::Add,
                context.caller_instance(),
                &provider.config.membership_admin_callers,
                &request,
                invalid,
            )
            .await
            .map_err(|error| failure(&error))
        })
    }
    fn remove_member(
        &self,
        context: InvocationContext,
        request: members::RemoveMemberRequest,
    ) -> NativeRequestFuture<members::OrganizationMembershipAdminRemoveMember> {
        let provider = self.clone();
        Box::pin(async move {
            provider.ready()?;
            let invalid = (!valid_membership_request(
                &request.idempotency_key,
                &request.organization_id,
                &request.subject,
            ))
            .then_some("invalid_request");
            storage::mutate(
                &provider.binding,
                storage::Mutation::Remove,
                context.caller_instance(),
                &provider.config.membership_admin_callers,
                &request,
                invalid,
            )
            .await
            .map_err(|error| failure(&error))
        })
    }
    fn list_members(
        &self,
        context: InvocationContext,
        request: members::ListMembersRequest,
    ) -> NativeRequestFuture<members::OrganizationMembershipAdminListMembers> {
        let provider = self.clone();
        Box::pin(async move {
            if exact_caller(
                context.caller_instance(),
                &provider.config.membership_admin_callers,
            )
            .is_none()
            {
                return Ok(Err(members::ListMembersError::Forbidden));
            }
            if !valid_page(request.limit, request.cursor.as_deref()) {
                return Ok(Err(members::ListMembersError::InvalidPage));
            }
            if !valid_name(&request.organization_id, 256)
                || !request
                    .subject
                    .as_deref()
                    .is_none_or(|v| valid_name(v, 256))
            {
                return Ok(Err(members::ListMembersError::InvalidRequest));
            }
            provider.ready()?;
            storage::read(&provider.binding, storage::LIST_MEMBERS, &request)
                .await
                .map_err(|error| failure(&error))
        })
    }
}

impl directory::OrganizationDirectoryProvider for Provider {
    fn get_organization(
        &self,
        context: InvocationContext,
        request: directory::GetOrganizationRequest,
    ) -> NativeRequestFuture<directory::OrganizationDirectoryGetOrganization> {
        let provider = self.clone();
        Box::pin(async move {
            if exact_caller(
                context.caller_instance(),
                &provider.config.directory_callers,
            )
            .is_none()
            {
                return Ok(Err(directory::GetOrganizationError::Forbidden));
            }
            if !valid_name(&request.organization_id, 256) {
                return Ok(Err(directory::GetOrganizationError::InvalidRequest));
            }
            provider.ready()?;
            storage::read(&provider.binding, storage::GET, &request)
                .await
                .map_err(|error| failure(&error))
        })
    }
    fn list_for_subject(
        &self,
        context: InvocationContext,
        request: directory::ListForSubjectRequest,
    ) -> NativeRequestFuture<directory::OrganizationDirectoryListForSubject> {
        let provider = self.clone();
        Box::pin(async move {
            if exact_caller(
                context.caller_instance(),
                &provider.config.directory_callers,
            )
            .is_none()
            {
                return Ok(Err(directory::ListForSubjectError::Forbidden));
            }
            if !valid_name(&request.subject, 256)
                || !valid_page(request.limit, request.after.as_deref())
            {
                return Ok(Err(directory::ListForSubjectError::InvalidRequest));
            }
            provider.ready()?;
            storage::read(&provider.binding, storage::LIST_FOR_SUBJECT, &request)
                .await
                .map_err(|error| failure(&error))
        })
    }
}

impl membership::OrganizationMembershipProvider for Provider {
    fn check_membership(
        &self,
        _context: InvocationContext,
        request: membership::CheckMembershipRequest,
    ) -> NativeRequestFuture<membership::OrganizationMembership> {
        let provider = self.clone();
        Box::pin(async move {
            if !valid_name(&request.organization_id, 256) || !valid_name(&request.subject, 256) {
                return Ok(Err(membership::CheckMembershipError::InvalidRequest));
            }
            provider.ready()?;
            storage::read(&provider.binding, storage::CHECK, &request)
                .await
                .map_err(|error| failure(&error))
        })
    }
}
