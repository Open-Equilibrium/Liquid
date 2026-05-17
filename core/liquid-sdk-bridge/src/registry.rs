//! Minimal workspace metadata registry.
//!
//! The Phase-1 bridge needs a place to record "this workspace exists,
//! is named X, was created by P at time T" so `list_workspaces`
//! has something to return. The trait is generic so a Phase-3
//! distributed deployment can swap in a Postgres-backed registry
//! without touching the call sites.
//!
//! Ships two implementations:
//!
//! - [`InMemoryWorkspaceRegistry`] — test / dev backend.
//! - [`FilesystemWorkspaceRegistry`] — durable Phase-1 backend
//!   persisting to `<root>/workspaces.toml` (atomic tmp-then-rename
//!   writes, same idiom as `liquid-vcs::FilesystemContentStore`
//!   per ADR-001). The CLI (M6.5) needs persistence across
//!   process restarts.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
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

/// Process-local in-memory implementation. Phase-1 test / dev
/// backend; production uses [`FilesystemWorkspaceRegistry`].
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

/// Filesystem-backed `WorkspaceRegistry`.
///
/// Layout (per `IMPLEMENTATION_PLAN.md §9`):
///
/// ```text
/// <root>/
///   workspaces.toml      # one [[workspaces]] table per workspace
/// ```
///
/// Atomic writes use the standard tmp-then-rename idiom (same
/// pattern as `liquid-vcs::FilesystemContentStore` per ADR-001).
/// Records live in both an in-memory cache (so `list` is constant-
/// time relative to disk) and on disk (so they survive a process
/// restart — the M6.5 CLI calls this layer between independent
/// process invocations).
#[derive(Debug)]
pub struct FilesystemWorkspaceRegistry {
    root: PathBuf,
    cache: Mutex<Vec<WorkspaceRecord>>,
}

impl FilesystemWorkspaceRegistry {
    /// Open (or initialise) a filesystem-backed registry rooted at
    /// `root`. Loads `<root>/workspaces.toml` into the cache;
    /// absence of the file is fine (empty registry).
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|e| io_err("create root", &e))?;
        let file = root.join("workspaces.toml");
        let cache = if file.exists() {
            let text = fs::read_to_string(&file).map_err(|e| io_err("read registry", &e))?;
            let parsed: WorkspacesFile = toml::from_str(&text)
                .map_err(|e| LiquidError::InvalidInput(format!("workspaces.toml parse: {e}")))?;
            parsed.workspaces
        } else {
            Vec::new()
        };
        Ok(Self {
            root,
            cache: Mutex::new(cache),
        })
    }

    /// Path to the on-disk registry file (for tests / diagnostics).
    #[must_use]
    pub fn registry_path(&self) -> PathBuf {
        self.root.join("workspaces.toml")
    }

    /// Acquire the cache lock, transparently recovering from poison.
    /// See [`InMemoryWorkspaceRegistry::lock_records`] for the
    /// rationale.
    fn lock_cache(&self) -> MutexGuard<'_, Vec<WorkspaceRecord>> {
        self.cache.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Serialise `records` and atomically replace the on-disk file.
    fn flush_locked(&self, records: &[WorkspaceRecord]) -> Result<()> {
        let payload = WorkspacesFile {
            workspaces: records.to_vec(),
        };
        let text = toml::to_string(&payload)
            .map_err(|e| LiquidError::InvalidInput(format!("workspaces.toml encode: {e}")))?;
        atomic_write(&self.registry_path(), text.as_bytes())
    }
}

#[async_trait]
impl WorkspaceRegistry for FilesystemWorkspaceRegistry {
    /// Insert a record and persist the snapshot to disk.
    ///
    /// **Phase-1 concurrency caveat.** The in-memory cache lock is
    /// dropped *before* `flush_locked` writes the snapshot to disk
    /// (so a slow disk does not block the cache against concurrent
    /// readers). Two tasks calling `register` simultaneously inside
    /// the same process therefore race on the on-disk write order —
    /// the second snapshot wins but both registrations remain in
    /// memory, so a subsequent process restart may see only one.
    /// The Phase-1 CLI is single-process current-thread (see
    /// `IMPLEMENTATION_PLAN.md §5.6`), so this race is unreachable
    /// today; the file-locked variant lands with the Phase-3 server
    /// process (TASK aligned with the M18 distributed-event-bus
    /// work — `IMPLEMENTATION_PLAN.md §8`).
    async fn register(&self, record: WorkspaceRecord) -> Result<()> {
        let snapshot = {
            let mut guard = self.lock_cache();
            if guard.iter().any(|r| r.id == record.id) {
                let id = record.id;
                return Err(LiquidError::InvalidInput(format!(
                    "workspace already registered: {id}"
                )));
            }
            guard.push(record);
            guard.clone()
        };
        self.flush_locked(&snapshot)
    }

    async fn list(&self) -> Result<Vec<WorkspaceSummary>> {
        let guard = self.lock_cache();
        let mut out: Vec<WorkspaceSummary> =
            guard.iter().cloned().map(WorkspaceSummary::from).collect();
        out.sort_by(|a, b| b.created_unix.cmp(&a.created_unix));
        Ok(out)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct WorkspacesFile {
    #[serde(default)]
    workspaces: Vec<WorkspaceRecord>,
}

/// Atomic write + Unix mode 0600 clamp.
///
/// `workspaces.toml` records `{id, name, created_by, created_unix}`
/// for every workspace on the host. Owner-only access prevents a
/// local attacker from enumerating the workspace ID space (which
/// `delete_workspace` already takes pains to keep behind an
/// anti-enumeration permission gate per §4.5).
fn atomic_write(target: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| io_err("create parent", &e))?;
    }
    let mut tmp = target.to_path_buf();
    tmp.set_extension("toml.tmp");
    {
        let mut f = fs::File::create(&tmp).map_err(|e| io_err("create tmp", &e))?;
        f.write_all(bytes).map_err(|e| io_err("write tmp", &e))?;
        f.sync_all().map_err(|e| io_err("sync tmp", &e))?;
    }
    fs::rename(&tmp, target).map_err(|e| io_err("rename", &e))?;
    restrict_perms(target)
}

#[cfg(unix)]
fn restrict_perms(target: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(target, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| io_err("chmod 0600", &e))
}

#[cfg(not(unix))]
fn restrict_perms(_target: &Path) -> Result<()> {
    Ok(())
}

fn io_err(stage: &str, e: &std::io::Error) -> LiquidError {
    LiquidError::InvalidInput(format!("workspace registry I/O ({stage}): {e}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn record(name: &str) -> WorkspaceRecord {
        WorkspaceRecord {
            id: WorkspaceId::new(),
            name: name.to_string(),
            created_by: PrincipalId::new_user(),
            created_unix: 0,
        }
    }

    #[tokio::test]
    async fn in_memory_register_rejects_duplicate_workspace_id() {
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
        let listed = r.list().await.expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, first.id);
        assert_eq!(listed[0].name, "alpha");
    }

    #[tokio::test]
    async fn in_memory_list_sorts_newest_first_by_created_unix() {
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

    #[tokio::test]
    async fn filesystem_persists_across_open_calls() {
        let dir = TempDir::new().expect("tempdir");
        let first = record("alpha");
        let second = WorkspaceRecord {
            id: WorkspaceId::new(),
            name: "beta".into(),
            created_by: PrincipalId::new_user(),
            created_unix: 50,
        };

        {
            let r = FilesystemWorkspaceRegistry::open(dir.path()).expect("open #1");
            r.register(first.clone()).await.expect("insert first");
            r.register(second.clone()).await.expect("insert second");
        }

        let r2 = FilesystemWorkspaceRegistry::open(dir.path()).expect("open #2");
        let listed = r2.list().await.expect("list");
        assert_eq!(listed.len(), 2, "both records must survive re-open");
        // first.created_unix = 0, second.created_unix = 50 ⇒ second newest
        assert_eq!(listed[0].id, second.id);
        assert_eq!(listed[1].id, first.id);
    }

    #[tokio::test]
    async fn filesystem_register_rejects_duplicate_workspace_id() {
        let dir = TempDir::new().expect("tempdir");
        let r = FilesystemWorkspaceRegistry::open(dir.path()).expect("open");
        let first = record("alpha");
        r.register(first.clone()).await.expect("first ok");
        let err = r
            .register(first.clone())
            .await
            .expect_err("duplicate must fail");
        assert!(matches!(err, LiquidError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn filesystem_open_rejects_malformed_toml() {
        let dir = TempDir::new().expect("tempdir");
        fs::write(dir.path().join("workspaces.toml"), "not = valid toml [[").expect("seed");
        let err = FilesystemWorkspaceRegistry::open(dir.path()).expect_err("malformed must fail");
        assert!(
            matches!(err, LiquidError::InvalidInput(_)),
            "malformed toml must surface as InvalidInput, got {err:?}"
        );
    }

    #[tokio::test]
    async fn filesystem_open_surfaces_io_err_when_root_cannot_be_created() {
        // Cover the `io_err` helper: `fs::create_dir_all` fails when
        // the requested root traverses through a regular file (you
        // cannot `mkdir` inside a file). The error must map to
        // `LiquidError::InvalidInput` via the
        // `io_err("create root", _)` path — otherwise that helper +
        // its two-line body is unreachable from the happy-path
        // tests.
        let dir = TempDir::new().expect("tempdir");
        let file_path = dir.path().join("not-a-dir");
        fs::write(&file_path, b"i am a file").expect("seed file");
        let through_file = file_path.join("subdir");
        let err = FilesystemWorkspaceRegistry::open(&through_file)
            .expect_err("create_dir_all must fail when parent is a file");
        match err {
            LiquidError::InvalidInput(msg) => {
                assert!(
                    msg.contains("workspace registry I/O"),
                    "must route through io_err helper, got: {msg}"
                );
                assert!(
                    msg.contains("create root"),
                    "must name the failing stage, got: {msg}"
                );
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn filesystem_registry_path_resolves_under_root() {
        let dir = TempDir::new().expect("tempdir");
        let r = FilesystemWorkspaceRegistry::open(dir.path()).expect("open");
        let path = r.registry_path();
        assert!(path.starts_with(dir.path()));
        assert_eq!(
            path.file_name().and_then(|s| s.to_str()),
            Some("workspaces.toml")
        );
    }
}
