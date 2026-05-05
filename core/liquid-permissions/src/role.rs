use serde::{Deserialize, Serialize};

use liquid_core::{Action, Resource};

/// Built-in RBAC roles. Phase 1 hardcodes the role → permission matrix here;
/// Phase 3 will add a `CustomRole(RoleId)` variant whose permission set is
/// configurable at runtime via [`PermissionIndex::grant`] (re-introduced then).
///
/// See `IMPLEMENTATION_PLAN.md` §9 (`liquid-permissions`) for the matrix
/// description and §5.3 for the M3 plan-level success criterion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltInRole {
    /// Full power within a workspace: every action on every resource.
    WorkspaceOwner,
    /// Reads everything; writes pages and app instances; cannot Admin.
    WorkspaceMember,
    /// Read-only access to a specific app instance (scope required).
    AppViewer,
    /// Read + Write access to a specific app instance (scope required).
    AppEditor,
    /// Marker role for non-human principals. Grants no permissions on its own;
    /// agents derive their authority from additional role bindings (per §9:
    /// "cannot exceed authorising principal").
    Agent,
}

impl BuiltInRole {
    /// `true` if this role permits `action` on `resource`, given that the
    /// binding's scope (if any) already matches the resource. Scope matching
    /// is the index's job, not this function's.
    pub fn permits(&self, action: Action, resource: &Resource) -> bool {
        use Action::{Admin, Delete, Read, Write};
        use BuiltInRole::{Agent, AppEditor, AppViewer, WorkspaceMember, WorkspaceOwner};

        match self {
            WorkspaceOwner => true,
            WorkspaceMember => match action {
                Read => true,
                Write | Delete => matches!(
                    resource,
                    Resource::Page(_) | Resource::AppInstance(_) | Resource::Component(_)
                ),
                Admin => false,
            },
            AppViewer => {
                matches!(action, Read)
                    && matches!(resource, Resource::AppInstance(_) | Resource::Component(_))
            }
            AppEditor => {
                matches!(action, Read | Write)
                    && matches!(resource, Resource::AppInstance(_) | Resource::Component(_))
            }
            Agent => false,
        }
    }

    /// `true` if this role must be assigned with a non-`None` `scope` (i.e.
    /// the binding only applies to a specific app instance).
    pub fn requires_scope(&self) -> bool {
        matches!(self, Self::AppViewer | Self::AppEditor)
    }
}
