use serde::{Deserialize, Serialize};

use crate::{AppInstanceId, ComponentId, PageId, WorkspaceId};

/// Capability verbs used in RBAC checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Read,
    Write,
    Delete,
    Admin,
}

/// The target of a permission check.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum Resource {
    Workspace(WorkspaceId),
    AppInstance(AppInstanceId),
    Component(ComponentId),
    Page(PageId),
    Field(String),
}
