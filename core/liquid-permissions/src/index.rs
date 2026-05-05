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
    /// Must be O(1) under load — this is on the hot path of every bridge call.
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
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct Binding {
    workspace: WorkspaceId,
    principal: PrincipalId,
    role: BuiltInRole,
    scope: Option<Resource>,
}

/// In-memory implementation of [`PermissionIndex`]. Phase-1 only — the disk-
/// backed TOML variant called for in `IMPLEMENTATION_PLAN.md` §5.3 is queued
/// as TASK-007.
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
        for binding in map.iter() {
            if binding.principal != principal {
                continue;
            }
            if !workspace_matches(binding.workspace, &resource) {
                continue;
            }
            if !scope_matches(binding.scope.as_ref(), &resource) {
                continue;
            }
            if binding.role.permits(action, &resource) {
                return Ok(true);
            }
        }
        Ok(false)
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
/// same workspace. Most resources (`AppInstance`, `Component`, `Page`,
/// `Field`) are addressed by globally-unique UUIDs and are checked via their
/// UUIDs alone; the binding's `workspace` is then informational. The one
/// resource that carries a workspace identifier is `Resource::Workspace(_)` —
/// for those we match strictly so an Owner of workspace A doesn't get
/// authority over B.
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
