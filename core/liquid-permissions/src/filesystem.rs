//! On-disk implementation of [`crate::PermissionIndex`].
//!
//! Layout (per `IMPLEMENTATION_PLAN.md` §9):
//!
//! ```text
//! <root>/
//!   workspaces/
//!     <workspace_id>/
//!       permissions.toml      # role bindings for this workspace only
//! ```
//!
//! Atomic writes use the standard tmp-then-rename idiom (same pattern as
//! `liquid-vcs::FilesystemContentStore` per ADR-001). Bindings live both
//! in an in-memory cache (so `check` is the same O(n-bindings) it is for
//! the in-memory variant) and on disk (so they survive a process restart).
//!
//! TASK-007 — finishes M3's "disk-backed `PermissionIndex`" follow-up.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use async_trait::async_trait;
use liquid_core::{Action, LiquidError, PrincipalId, Resource, Result, WorkspaceId};
use serde::{Deserialize, Serialize};

use crate::index::{Binding, PermissionIndex};
use crate::role::BuiltInRole;

/// Filesystem-backed `PermissionIndex`.
///
/// Construct with [`FilesystemPermissionIndex::open`]; the constructor
/// scans `<root>/workspaces/*/permissions.toml` and loads every workspace
/// it finds. Mutations write the affected workspace's file atomically.
#[derive(Debug)]
pub struct FilesystemPermissionIndex {
    root: PathBuf,
    cache: Mutex<HashMap<WorkspaceId, HashSet<Binding>>>,
}

impl FilesystemPermissionIndex {
    /// Open (or initialise) a filesystem-backed index rooted at `root`.
    /// Loads every existing `<root>/workspaces/<id>/permissions.toml`
    /// into memory; absence of any of those files is fine (empty index).
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|e| io_err("create root", &e))?;
        let workspaces_dir = root.join("workspaces");
        let mut cache: HashMap<WorkspaceId, HashSet<Binding>> = HashMap::new();
        if workspaces_dir.exists() {
            for entry in
                fs::read_dir(&workspaces_dir).map_err(|e| io_err("read workspaces dir", &e))?
            {
                let entry = entry.map_err(|e| io_err("iterate workspaces dir", &e))?;
                if !entry
                    .file_type()
                    .map_err(|e| io_err("file type", &e))?
                    .is_dir()
                {
                    continue;
                }
                let workspace = parse_workspace_id(&entry.file_name())?;
                let path = entry.path().join("permissions.toml");
                if !path.exists() {
                    continue;
                }
                let bindings = load_workspace_file(&path)?
                    .into_iter()
                    .map(|disk| disk.into_binding(workspace))
                    .collect();
                cache.insert(workspace, bindings);
            }
        }
        Ok(Self {
            root,
            cache: Mutex::new(cache),
        })
    }

    /// Path to the on-disk file for `workspace` (for tests / diagnostics).
    pub fn workspace_path(&self, workspace: WorkspaceId) -> PathBuf {
        workspace_file(&self.root, workspace)
    }

    fn flush_workspace_locked(
        cache: &HashMap<WorkspaceId, HashSet<Binding>>,
        root: &Path,
        workspace: WorkspaceId,
    ) -> Result<()> {
        let bindings = cache.get(&workspace);
        let disk: Vec<DiskBinding> = bindings
            .map(|set| set.iter().cloned().map(DiskBinding::from).collect())
            .unwrap_or_default();
        let payload = WorkspaceFile { bindings: disk };
        let text = toml::to_string(&payload)
            .map_err(|e| LiquidError::InvalidInput(format!("permissions.toml encode: {e}")))?;
        atomic_write(&workspace_file(root, workspace), text.as_bytes())
    }
}

#[async_trait]
impl PermissionIndex for FilesystemPermissionIndex {
    async fn check(
        &self,
        principal: PrincipalId,
        action: Action,
        resource: Resource,
    ) -> Result<bool> {
        let cache = self.cache.lock().map_err(poisoned)?;
        for bindings in cache.values() {
            for binding in bindings {
                if binding.matches(principal, action, &resource) {
                    return Ok(true);
                }
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
        let mut cache = self.cache.lock().map_err(poisoned)?;
        cache.entry(workspace).or_default().insert(Binding {
            workspace,
            principal,
            role,
            scope,
        });
        Self::flush_workspace_locked(&cache, &self.root, workspace)
    }

    async fn revoke_role(
        &self,
        workspace: WorkspaceId,
        principal: PrincipalId,
        role: BuiltInRole,
        scope: Option<Resource>,
    ) -> Result<()> {
        let mut cache = self.cache.lock().map_err(poisoned)?;
        if let Some(set) = cache.get_mut(&workspace) {
            set.remove(&Binding {
                workspace,
                principal,
                role,
                scope,
            });
        }
        Self::flush_workspace_locked(&cache, &self.root, workspace)
    }
}

/// On-disk shape: the `workspace` is encoded by the file path, not by the
/// record, to avoid a single record being trusted in the wrong workspace's
/// file.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiskBinding {
    principal: PrincipalId,
    role: BuiltInRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scope: Option<Resource>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct WorkspaceFile {
    #[serde(default)]
    bindings: Vec<DiskBinding>,
}

impl DiskBinding {
    fn into_binding(self, workspace: WorkspaceId) -> Binding {
        Binding {
            workspace,
            principal: self.principal,
            role: self.role,
            scope: self.scope,
        }
    }
}

impl From<Binding> for DiskBinding {
    fn from(b: Binding) -> Self {
        Self {
            principal: b.principal,
            role: b.role,
            scope: b.scope,
        }
    }
}

fn workspace_file(root: &Path, workspace: WorkspaceId) -> PathBuf {
    root.join("workspaces")
        .join(workspace.to_string())
        .join("permissions.toml")
}

fn parse_workspace_id(name: &std::ffi::OsStr) -> Result<WorkspaceId> {
    let s = name
        .to_str()
        .ok_or_else(|| LiquidError::InvalidInput("non-UTF-8 workspace directory name".into()))?;
    let uuid = uuid::Uuid::parse_str(s).map_err(|e| {
        LiquidError::InvalidInput(format!("workspace directory name is not a UUID: {s}: {e}"))
    })?;
    Ok(WorkspaceId(uuid))
}

fn load_workspace_file(path: &Path) -> Result<Vec<DiskBinding>> {
    let text = fs::read_to_string(path).map_err(|e| io_err("read permissions.toml", &e))?;
    let parsed: WorkspaceFile = toml::from_str(&text)
        .map_err(|e| LiquidError::InvalidInput(format!("permissions.toml parse: {e}")))?;
    Ok(parsed.bindings)
}

/// Atomic write + Unix mode 0600 clamp.
///
/// `permissions.toml` leaks the full role-binding table (principal
/// UUID → role → resource) and workspace membership. Owner-only
/// access prevents a local attacker from enumerating who has Admin
/// or who has access to which workspace.
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
#[allow(clippy::unnecessary_wraps)] // signature must match the Unix arm
fn restrict_perms(_target: &Path) -> Result<()> {
    Ok(())
}

fn io_err(stage: &str, e: &std::io::Error) -> LiquidError {
    LiquidError::InvalidInput(format!("permissions storage I/O ({stage}): {e}"))
}

fn poisoned<T>(_: PoisonError<T>) -> LiquidError {
    LiquidError::InvalidInput("permission index lock poisoned".into())
}
