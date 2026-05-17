use std::collections::HashSet;
use std::sync::{Mutex, PoisonError};

use async_trait::async_trait;
use liquid_core::{Action, LiquidError, PrincipalId, Resource, Result, WorkspaceId};

use crate::role::BuiltInRole;

/// Materialised RBAC index.
///
/// Trait shape mirrors `IMPLEMENTATION_PLAN.md` §4.2 with two Phase-1
/// adjustments documented there:
///
/// 1. Errors normalise to [`liquid_core::LiquidError`] (matches the §4.1
///    convention; no parallel `PermError` hierarchy).
/// 2. `grant(role, action, resource)` is omitted because Phase 1 ships a
///    hard-coded role → permission matrix (see [`BuiltInRole::permits`]).
///    Phase 3 will reintroduce `grant` for custom roles.
#[async_trait]
pub trait PermissionIndex: Send + Sync {
    /// `true` iff `principal` may perform `action` on `resource`.
    ///
    /// Phase-1 implementations are **`O(n_bindings)`** — both
    /// `InMemoryPermissionIndex` and `FilesystemPermissionIndex`
    /// scan a `HashSet<Binding>`. The materialised
    /// principal → action → resource index that brings this down to
    /// `O(1)` lands with Phase-3 Milestone 15 (see
    /// `IMPLEMENTATION_PLAN.md §7.3` — "Distributed permission
    /// index"). Callers should not assume `O(1)` today;
    /// `list_workspaces` in particular compounds to
    /// `O(n_workspaces × n_bindings)` per call.
    async fn check(
        &self,
        principal: PrincipalId,
        action: Action,
        resource: Resource,
    ) -> Result<bool>;

    /// Bind `principal` to `role` within `workspace`. For roles whose
    /// [`BuiltInRole::requires_scope`] is true, `scope` must be `Some(_)`;
    /// for workspace-wide roles, `scope` may be `None`.
    async fn assign_role(
        &self,
        workspace: WorkspaceId,
        principal: PrincipalId,
        role: BuiltInRole,
        scope: Option<Resource>,
    ) -> Result<()>;

    /// Reverse [`Self::assign_role`]. Idempotent — revoking a non-existent
    /// binding is a no-op.
    async fn revoke_role(
        &self,
        workspace: WorkspaceId,
        principal: PrincipalId,
        role: BuiltInRole,
        scope: Option<Resource>,
    ) -> Result<()>;
}

/// One row in the role-binding table.
///
/// `pub(crate)` so the on-disk variant in [`crate::filesystem`] can construct
/// and consume the same shape; not exposed in the public API.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) struct Binding {
    pub(crate) workspace: WorkspaceId,
    pub(crate) principal: PrincipalId,
    pub(crate) role: BuiltInRole,
    pub(crate) scope: Option<Resource>,
}

impl Binding {
    /// Whether this binding grants `principal` the right to perform `action`
    /// on `resource`. Encapsulates the workspace + scope + role-matrix check
    /// so both `InMemoryPermissionIndex` and `FilesystemPermissionIndex`
    /// share one definition.
    pub(crate) fn matches(
        &self,
        principal: PrincipalId,
        action: Action,
        resource: &Resource,
    ) -> bool {
        if self.principal != principal {
            return false;
        }
        if !workspace_matches(self.workspace, resource) {
            return false;
        }
        if !scope_matches(self.scope.as_ref(), resource) {
            return false;
        }
        self.role.permits(action, resource)
    }
}

/// In-memory implementation of [`PermissionIndex`]. Use this in tests
/// and dev mode where persistence is not required. The durable TOML-
/// backed sibling [`crate::FilesystemPermissionIndex`]
/// (`IMPLEMENTATION_PLAN.md §5.3`) ships the same trait surface for
/// Phase-1 production deployments; both backends share the
/// [`Binding::matches`] check.
#[derive(Debug, Default)]
pub struct InMemoryPermissionIndex {
    bindings: Mutex<HashSet<Binding>>,
}

impl InMemoryPermissionIndex {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl PermissionIndex for InMemoryPermissionIndex {
    async fn check(
        &self,
        principal: PrincipalId,
        action: Action,
        resource: Resource,
    ) -> Result<bool> {
        let map = self.bindings.lock().map_err(poisoned)?;
        Ok(map
            .iter()
            .any(|binding| binding.matches(principal, action, &resource)))
    }

    async fn assign_role(
        &self,
        workspace: WorkspaceId,
        principal: PrincipalId,
        role: BuiltInRole,
        scope: Option<Resource>,
    ) -> Result<()> {
        if role.requires_scope() && scope.is_none() {
            return Err(LiquidError::InvalidInput(format!(
                "role {role:?} requires a resource scope"
            )));
        }
        let mut map = self.bindings.lock().map_err(poisoned)?;
        map.insert(Binding {
            workspace,
            principal,
            role,
            scope,
        });
        Ok(())
    }

    async fn revoke_role(
        &self,
        workspace: WorkspaceId,
        principal: PrincipalId,
        role: BuiltInRole,
        scope: Option<Resource>,
    ) -> Result<()> {
        let mut map = self.bindings.lock().map_err(poisoned)?;
        map.remove(&Binding {
            workspace,
            principal,
            role,
            scope,
        });
        Ok(())
    }
}

/// A workspace-scoped binding only applies to resources that belong to the
/// same workspace. For `Resource::Workspace(_)` we match strictly so an
/// Owner of workspace A doesn't get authority over B. For
/// `Resource::AppInstance`, `Component`, and `Page` we match
/// workspace-agnostically and rely on the globally-unique UUID assumption
/// (see `IMPLEMENTATION_PLAN.md §4.2` "Tenant-isolation note for resource
/// ids"): every UUID is `Uuid::new_v4()` and never reused across
/// workspaces, so two workspaces cannot share the same id.
///
/// `Resource::Field(String)` is also matched workspace-agnostically and is
/// the one variant that does NOT carry a globally-unique guarantee
/// (multiple workspaces can legitimately use the same field name). No
/// Phase-1 `BuiltInRole` grants any permission on `Field`, so the surface
/// is currently unreachable; the Phase-3 follow-up flagged in
/// `IMPLEMENTATION_PLAN.md §4.2` must either qualify `Field` with a
/// `ComponentId` or make this arm workspace-strict before any new role
/// binds a `Field`.
fn workspace_matches(binding_ws: WorkspaceId, resource: &Resource) -> bool {
    match resource {
        Resource::Workspace(target) => binding_ws == *target,
        _ => true,
    }
}

fn scope_matches(scope: Option<&Resource>, resource: &Resource) -> bool {
    match scope {
        None => true,
        Some(s) => s == resource,
    }
}

fn poisoned<T>(_: PoisonError<T>) -> LiquidError {
    LiquidError::InvalidInput("permission index lock poisoned".into())
}
