use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use bytes::Bytes;
use liquid_core::{
    CommitId, LiquidError, OperationId, PrincipalId, Result, StorePath, WorkspaceId,
};

use crate::{ContentStore, Operation, OperationKind};

/// On-disk `ContentStore` implementation.
///
/// Each workspace lives at `<root>/<workspace_id>/`:
///
/// ```text
/// <root>/<workspace_id>/files/<store_path>   # raw file bytes
/// <root>/<workspace_id>/op_log.jsonl         # newline-delimited Operation JSON
/// ```
///
/// Atomic writes use the standard tmp-then-rename idiom. See
/// `docs/adr/001-jujutsu-pinning.md` for why this is the Phase 1 default and
/// what the upgrade path to a `JujutsuContentStore` looks like.
///
/// **Phase-1 concurrency caveat.** Every method here is declared
/// `async` to fit the `ContentStore` trait, but the body uses
/// synchronous `std::fs::*` calls and holds [`Self::write_lock`]
/// across `f.sync_all()`. The Phase-1 CLI is single-process
/// current-thread (`tokio::runtime::Builder::new_current_thread`),
/// so a blocking `sync_all` only stalls the calling thread — there
/// is no executor pool to starve. When the same crate is embedded
/// in the multi-task Phase-3 server, every storage call here MUST
/// be moved under `tokio::task::spawn_blocking` (or replaced with
/// `tokio::fs::*` + a real file-lock primitive); the Jujutsu
/// backend planned in TASK-004 handles its own async I/O so the
/// server only needs to gate the in-memory + JSONL variants.
#[derive(Debug)]
pub struct FilesystemContentStore {
    root: PathBuf,
    /// Serialises all writes within a single process instance. Cross-process
    /// coordination would require a separate file lock; out of scope for M2.
    write_lock: Mutex<()>,
}

impl FilesystemContentStore {
    /// Open a store rooted at `root`. The directory is created if it does
    /// not exist; the store does not assume exclusive ownership of the path.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)
            .map_err(|e| io_err(&format!("create root {}", root.display()), &e))?;
        Ok(Self {
            root,
            write_lock: Mutex::new(()),
        })
    }

    /// Returns the on-disk root for inspection / tests.
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn workspace_dir(&self, ws: WorkspaceId) -> PathBuf {
        self.root.join(ws.to_string())
    }

    fn files_dir(&self, ws: WorkspaceId) -> PathBuf {
        self.workspace_dir(ws).join("files")
    }

    fn file_path(&self, ws: WorkspaceId, path: &StorePath) -> PathBuf {
        self.files_dir(ws).join(path.as_str())
    }

    fn op_log_path(&self, ws: WorkspaceId) -> PathBuf {
        self.workspace_dir(ws).join("op_log.jsonl")
    }

    fn read_op_log(&self, ws: WorkspaceId) -> Result<Vec<Operation>> {
        let path = self.op_log_path(ws);
        match File::open(&path) {
            Ok(file) => {
                let mut out = Vec::new();
                for (idx, line) in BufReader::new(file).lines().enumerate() {
                    let line = line
                        .map_err(|e| io_err(&format!("read {} line {idx}", path.display()), &e))?;
                    if line.trim().is_empty() {
                        continue;
                    }
                    let op: Operation = serde_json::from_str(&line).map_err(|e| {
                        LiquidError::InvalidInput(format!(
                            "corrupt op log line {idx} in {}: {e}",
                            path.display()
                        ))
                    })?;
                    out.push(op);
                }
                Ok(out)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(LiquidError::NotFound(format!("workspace {ws}")))
            }
            Err(e) => Err(io_err(&format!("open {}", path.display()), &e)),
        }
    }

    fn append_op(&self, ws: WorkspaceId, op: &Operation) -> Result<()> {
        let dir = self.workspace_dir(ws);
        fs::create_dir_all(&dir).map_err(|e| io_err(&format!("mkdir {}", dir.display()), &e))?;
        let path = self.op_log_path(ws);
        let mut line = serde_json::to_string(op)
            .map_err(|e| LiquidError::InvalidInput(format!("encode op: {e}")))?;
        line.push('\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| io_err(&format!("open {}", path.display()), &e))?;
        file.write_all(line.as_bytes())
            .map_err(|e| io_err(&format!("append {}", path.display()), &e))?;
        file.sync_all()
            .map_err(|e| io_err(&format!("fsync {}", path.display()), &e))?;
        Ok(())
    }
}

fn write_file_atomic(full_path: &Path, content: &[u8]) -> Result<()> {
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| io_err(&format!("mkdir {}", parent.display()), &e))?;
    }
    let tmp = full_path.with_extension("liquid-tmp");
    {
        let mut f =
            File::create(&tmp).map_err(|e| io_err(&format!("create {}", tmp.display()), &e))?;
        f.write_all(content)
            .map_err(|e| io_err(&format!("write {}", tmp.display()), &e))?;
        f.sync_all()
            .map_err(|e| io_err(&format!("fsync {}", tmp.display()), &e))?;
    }
    fs::rename(&tmp, full_path).map_err(|e| {
        io_err(
            &format!("rename {} -> {}", tmp.display(), full_path.display()),
            &e,
        )
    })?;
    Ok(())
}

fn delete_file(full_path: &Path) -> Result<()> {
    match fs::remove_file(full_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_err(&format!("remove {}", full_path.display()), &e)),
    }
}

#[async_trait]
impl ContentStore for FilesystemContentStore {
    async fn read(&self, workspace: WorkspaceId, path: &StorePath) -> Result<Bytes> {
        let ws_dir = self.workspace_dir(workspace);
        if !ws_dir.exists() {
            return Err(LiquidError::NotFound(format!("workspace {workspace}")));
        }
        let full = self.file_path(workspace, path);
        match fs::read(&full) {
            Ok(b) => Ok(Bytes::from(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(LiquidError::NotFound(format!("path {path}")))
            }
            Err(e) => Err(io_err(&format!("read {}", full.display()), &e)),
        }
    }

    async fn write(
        &self,
        workspace: WorkspaceId,
        path: &StorePath,
        content: Bytes,
        author: PrincipalId,
        message: &str,
    ) -> Result<CommitId> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| LiquidError::InvalidInput("filesystem write lock poisoned".into()))?;

        let full = self.file_path(workspace, path);
        let prev = match fs::read(&full) {
            Ok(b) => Some(Bytes::from(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(io_err(&format!("read {}", full.display()), &e)),
        };

        write_file_atomic(&full, content.as_ref())?;

        let kind = match prev {
            None => OperationKind::Create {
                path: path.clone(),
                content: content.clone(),
            },
            Some(prev) => OperationKind::Update {
                path: path.clone(),
                prev,
                content: content.clone(),
            },
        };
        let commit = CommitId::new();
        let op = Operation {
            id: OperationId::new(),
            commit,
            timestamp_unix_millis: now_millis(),
            author,
            message: message.to_owned(),
            kind,
        };
        self.append_op(workspace, &op)?;
        Ok(commit)
    }

    async fn operation_log(&self, workspace: WorkspaceId, limit: usize) -> Result<Vec<Operation>> {
        let log = self.read_op_log(workspace)?;
        Ok(log.into_iter().rev().take(limit).collect())
    }

    async fn undo(&self, workspace: WorkspaceId, op_id: OperationId) -> Result<CommitId> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| LiquidError::InvalidInput("filesystem write lock poisoned".into()))?;

        let log = self.read_op_log(workspace)?;
        let target = log
            .iter()
            .find(|o| o.id == op_id)
            .ok_or_else(|| LiquidError::NotFound(format!("operation {op_id}")))?
            .clone();

        match &target.kind {
            OperationKind::Create { path, .. } => {
                delete_file(&self.file_path(workspace, path))?;
            }
            OperationKind::Update { path, prev, .. } | OperationKind::Delete { path, prev } => {
                write_file_atomic(&self.file_path(workspace, path), prev.as_ref())?;
            }
            OperationKind::Undo { .. } => {
                return Err(LiquidError::InvalidInput("cannot undo an undo".into()));
            }
        }
        let commit = CommitId::new();
        let op = Operation {
            id: OperationId::new(),
            commit,
            timestamp_unix_millis: now_millis(),
            author: target.author,
            message: format!("undo: {}", target.message),
            kind: OperationKind::Undo { target: op_id },
        };
        self.append_op(workspace, &op)?;
        Ok(commit)
    }

    async fn list(&self, workspace: WorkspaceId, prefix: &StorePath) -> Result<Vec<StorePath>> {
        let ws_dir = self.workspace_dir(workspace);
        if !ws_dir.exists() {
            return Err(LiquidError::NotFound(format!("workspace {workspace}")));
        }
        let files_dir = self.files_dir(workspace);
        let mut out = Vec::new();
        walk(&files_dir, &files_dir, &mut out)?;
        let prefix_str = prefix.as_str();
        let dir_prefix = format!("{prefix_str}/");
        Ok(out
            .into_iter()
            .filter(|p| p.as_str().starts_with(&dir_prefix) || p.as_str() == prefix_str)
            .collect())
    }
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<StorePath>) -> Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(io_err(&format!("read_dir {}", dir.display()), &e)),
    };
    for entry in entries {
        let entry = entry.map_err(|e| io_err(&format!("read_dir entry {}", dir.display()), &e))?;
        let ft = entry
            .file_type()
            .map_err(|e| io_err(&format!("file_type {}", entry.path().display()), &e))?;
        let path = entry.path();
        if ft.is_dir() {
            walk(root, &path, out)?;
        } else if ft.is_file() {
            // skip *.liquid-tmp leftover files
            if path.extension().is_some_and(|e| e == "liquid-tmp") {
                continue;
            }
            let rel = path.strip_prefix(root).map_err(|e| {
                LiquidError::InvalidInput(format!("strip_prefix {}: {e}", path.display()))
            })?;
            let rel_str = rel
                .to_str()
                .ok_or_else(|| {
                    LiquidError::InvalidInput(format!("non-UTF8 path {}", rel.display()))
                })?
                .replace(std::path::MAIN_SEPARATOR, "/");
            let sp = StorePath::new(rel_str)?;
            out.push(sp);
        }
    }
    Ok(())
}

fn io_err(ctx: &str, e: &std::io::Error) -> LiquidError {
    LiquidError::InvalidInput(format!("{ctx}: {e}"))
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}
