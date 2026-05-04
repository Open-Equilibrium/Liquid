use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use bytes::Bytes;
use liquid_core::{
    CommitId, LiquidError, OperationId, PrincipalId, Result, StorePath, WorkspaceId,
};

use crate::{ContentStore, Operation, OperationKind};

#[derive(Debug, Default)]
struct WorkspaceState {
    files: HashMap<StorePath, Bytes>,
    op_log: Vec<Operation>,
}

/// In-memory `ContentStore`. Persists nothing; safe to share across threads
/// via `Arc<InMemoryContentStore>`. Intended for tests and Phase 1 dev mode.
#[derive(Debug, Default)]
pub struct InMemoryContentStore {
    workspaces: Mutex<HashMap<WorkspaceId, WorkspaceState>>,
}

impl InMemoryContentStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ContentStore for InMemoryContentStore {
    async fn read(&self, workspace: WorkspaceId, path: &StorePath) -> Result<Bytes> {
        let map = self.workspaces.lock().map_err(poisoned)?;
        let ws = map
            .get(&workspace)
            .ok_or_else(|| not_found_workspace(workspace))?;
        ws.files
            .get(path)
            .cloned()
            .ok_or_else(|| LiquidError::NotFound(format!("path {path}")))
    }

    async fn write(
        &self,
        workspace: WorkspaceId,
        path: &StorePath,
        content: Bytes,
        author: PrincipalId,
        message: &str,
    ) -> Result<CommitId> {
        let mut map = self.workspaces.lock().map_err(poisoned)?;
        let ws = map.entry(workspace).or_default();
        let prev = ws.files.get(path).cloned();
        let commit = CommitId::new();
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
        ws.files.insert(path.clone(), content);
        ws.op_log.push(Operation {
            id: OperationId::new(),
            commit,
            timestamp_unix_millis: now_millis(),
            author,
            message: message.to_owned(),
            kind,
        });
        Ok(commit)
    }

    async fn operation_log(&self, workspace: WorkspaceId, limit: usize) -> Result<Vec<Operation>> {
        let map = self.workspaces.lock().map_err(poisoned)?;
        let ws = map
            .get(&workspace)
            .ok_or_else(|| not_found_workspace(workspace))?;
        let mut log: Vec<Operation> = ws.op_log.iter().rev().take(limit).cloned().collect();
        log.shrink_to_fit();
        Ok(log)
    }

    async fn undo(&self, workspace: WorkspaceId, op_id: OperationId) -> Result<CommitId> {
        let mut map = self.workspaces.lock().map_err(poisoned)?;
        let ws = map
            .get_mut(&workspace)
            .ok_or_else(|| not_found_workspace(workspace))?;
        let target = ws
            .op_log
            .iter()
            .find(|o| o.id == op_id)
            .ok_or_else(|| LiquidError::NotFound(format!("operation {op_id}")))?
            .clone();
        match &target.kind {
            OperationKind::Create { path, .. } => {
                ws.files.remove(path);
            }
            OperationKind::Update { path, prev, .. } | OperationKind::Delete { path, prev } => {
                ws.files.insert(path.clone(), prev.clone());
            }
            OperationKind::Undo { .. } => {
                return Err(LiquidError::InvalidInput("cannot undo an undo".into()));
            }
        }
        let commit = CommitId::new();
        ws.op_log.push(Operation {
            id: OperationId::new(),
            commit,
            timestamp_unix_millis: now_millis(),
            author: target.author,
            message: format!("undo: {}", target.message),
            kind: OperationKind::Undo { target: op_id },
        });
        Ok(commit)
    }

    async fn list(&self, workspace: WorkspaceId, prefix: &StorePath) -> Result<Vec<StorePath>> {
        let map = self.workspaces.lock().map_err(poisoned)?;
        let ws = map
            .get(&workspace)
            .ok_or_else(|| not_found_workspace(workspace))?;
        let dir_prefix = format!("{}/", prefix.as_str());
        Ok(ws
            .files
            .keys()
            .filter(|p| p.as_str().starts_with(&dir_prefix) || p.as_str() == prefix.as_str())
            .cloned()
            .collect())
    }
}

fn poisoned<T>(_: PoisonError<T>) -> LiquidError {
    LiquidError::InvalidInput("workspace lock poisoned".into())
}

fn not_found_workspace(ws: WorkspaceId) -> LiquidError {
    LiquidError::NotFound(format!("workspace {ws}"))
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}
