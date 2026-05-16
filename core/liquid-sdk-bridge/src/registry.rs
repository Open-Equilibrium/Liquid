//! Minimal workspace metadata registry.
//!
//! The Phase-1 bridge needs a place to record "this workspace exists,
//! is named X, was created by P at time T" so `list_workspaces`
//! has something to return. The trait is generic so a Phase-3
//! distributed deployment can swap in a Postgres-backed registry
//! without touching the call sites.
//!
//! Phase-1 ships only the in-memory variant — the on-disk variant
//! is a follow-up that pairs with M6.5's CLI persistence work
//! (durable across process restarts). Persistence of role bindings
//! already lives in `liquid_permissions::FilesystemPermissionIndex`,
//! so a node restart loses workspace *names* but not authority.

use std::sync::{Mutex, PoisonError};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use liquid_core::{LiquidError, PrincipalId, Result, WorkspaceId};

use crate::types::WorkspaceSummary;

/// One row in the workspace registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRecord {
    pub id: WorkspaceId,
    pub name: String,
    pub created_by: PrincipalId,
    pub created_unix: u64,
}

impl From<WorkspaceRecord> for WorkspaceSummary {
    fn from(r: WorkspaceRecord) -> Self {
        Self {
            id: r.id,
            name: r.name,
            created_by: r.created_by,
            created_unix: r.created_unix,
        }
    }
}

/// Persists the existence + display metadata of every workspace.
///
/// Authority over a workspace lives in
/// `liquid_permissions::PermissionIndex`; this trait only records
/// the workspace's *identity*. Implementations must be `Send + Sync`
/// so the bridge can share an `Arc<dyn WorkspaceRegistry>` across
/// async tasks.
#[async_trait]
pub trait WorkspaceRegistry: Send + Sync {
    /// Insert a new workspace. Returns `LiquidError::InvalidInput` if a
    /// workspace with the same `id` already exists (Phase-1 IDs come
    /// from `Uuid::new_v4`, so duplicates indicate a programming
    /// mistake, not a collision).
    async fn register(&self, record: WorkspaceRecord) -> Result<()>;

    /// Return every registered workspace, newest first.
    async fn list(&self) -> Result<Vec<WorkspaceSummary>>;
}

/// Process-local in-memory implementation. Phase-1 default; the
/// Phase-3 disk-backed variant ships as a follow-up.
#[derive(Debug, Default)]
pub struct InMemoryWorkspaceRegistry {
    records: Mutex<Vec<WorkspaceRecord>>,
}

impl InMemoryWorkspaceRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl WorkspaceRegistry for InMemoryWorkspaceRegistry {
    async fn register(&self, record: WorkspaceRecord) -> Result<()> {
        let mut guard = self.records.lock().map_err(poisoned)?;
        if guard.iter().any(|r| r.id == record.id) {
            return Err(LiquidError::InvalidInput(format!(
                "workspace already registered: {}",
                record.id
            )));
        }
        guard.push(record);
        Ok(())
    }

    async fn list(&self) -> Result<Vec<WorkspaceSummary>> {
        let guard = self.records.lock().map_err(poisoned)?;
        let mut out: Vec<WorkspaceSummary> =
            guard.iter().cloned().map(WorkspaceSummary::from).collect();
        out.sort_by(|a, b| b.created_unix.cmp(&a.created_unix));
        Ok(out)
    }
}

fn poisoned<T>(_: PoisonError<T>) -> LiquidError {
    LiquidError::InvalidInput("workspace registry lock poisoned".into())
}
