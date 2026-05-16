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

use std::sync::{Mutex, MutexGuard, PoisonError};

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

    /// Acquire the records lock, transparently recovering from poison.
    /// If a previous holder panicked the inner `Vec` is at worst stale
    /// — re-reading the list returns existing rows in their original
    /// order, which the caller can survive (Mirrors the
    /// `CachedContentStore::lock_index` precedent and keeps the
    /// registry Absolute-Rule-1 compliant without an unreachable
    /// `LiquidError::InvalidInput("…")` error path that codecov
    /// would otherwise flag as uncovered.)
    fn lock_records(&self) -> MutexGuard<'_, Vec<WorkspaceRecord>> {
        self.records.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[async_trait]
impl WorkspaceRegistry for InMemoryWorkspaceRegistry {
    async fn register(&self, record: WorkspaceRecord) -> Result<()> {
        let mut guard = self.lock_records();
        if guard.iter().any(|r| r.id == record.id) {
            let id = record.id;
            return Err(LiquidError::InvalidInput(format!(
                "workspace already registered: {id}"
            )));
        }
        guard.push(record);
        Ok(())
    }

    async fn list(&self) -> Result<Vec<WorkspaceSummary>> {
        let guard = self.lock_records();
        let mut out: Vec<WorkspaceSummary> =
            guard.iter().cloned().map(WorkspaceSummary::from).collect();
        out.sort_by(|a, b| b.created_unix.cmp(&a.created_unix));
        Ok(out)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn record(name: &str) -> WorkspaceRecord {
        WorkspaceRecord {
            id: WorkspaceId::new(),
            name: name.to_string(),
            created_by: PrincipalId::new_user(),
            created_unix: 0,
        }
    }

    #[tokio::test]
    async fn register_rejects_duplicate_workspace_id() {
        let r = InMemoryWorkspaceRegistry::new();
        let first = record("alpha");
        let dup = WorkspaceRecord {
            id: first.id,
            name: "beta".into(),
            created_by: PrincipalId::new_user(),
            created_unix: 1,
        };
        r.register(first.clone()).await.expect("first insert ok");
        let err = r.register(dup).await.expect_err("duplicate id must fail");
        assert!(matches!(err, LiquidError::InvalidInput(_)));
        // Only the first record survives.
        let listed = r.list().await.expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, first.id);
        assert_eq!(listed[0].name, "alpha");
    }

    #[tokio::test]
    async fn list_sorts_newest_first_by_created_unix() {
        let r = InMemoryWorkspaceRegistry::new();
        let mut older = record("older");
        older.created_unix = 100;
        let mut newer = record("newer");
        newer.created_unix = 200;
        r.register(older).await.expect("ok");
        r.register(newer).await.expect("ok");
        let listed = r.list().await.expect("list");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].name, "newer", "newest first");
        assert_eq!(listed[1].name, "older");
    }
}
