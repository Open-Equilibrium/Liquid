use async_trait::async_trait;
use bytes::Bytes;
use liquid_core::{CommitId, OperationId, PrincipalId, Result, StorePath, WorkspaceId};

use crate::Operation;

/// Versioned, workspace-scoped content store.
///
/// All errors normalise to [`liquid_core::LiquidError`] so cross-crate
/// boundaries stay uniform — the `IMPLEMENTATION_PLAN.md` §4.1 spec named a
/// dedicated `StoreError`, but the workspace-wide policy (CLAUDE.md) is that
/// every public function returns `Result<_, LiquidError>`.
#[async_trait]
pub trait ContentStore: Send + Sync {
    /// Read the current bytes at `path` in `workspace`. Returns
    /// `LiquidError::NotFound` if the workspace or path does not exist.
    async fn read(&self, workspace: WorkspaceId, path: &StorePath) -> Result<Bytes>;

    /// Atomically write `content` to `path`, recording an operation attributed
    /// to `author` with `message`. Returns the new [`CommitId`].
    async fn write(
        &self,
        workspace: WorkspaceId,
        path: &StorePath,
        content: Bytes,
        author: PrincipalId,
        message: &str,
    ) -> Result<CommitId>;

    /// Return up to `limit` operation log entries, newest first.
    async fn operation_log(&self, workspace: WorkspaceId, limit: usize) -> Result<Vec<Operation>>;

    /// Invert the operation identified by `op_id`. Returns the new
    /// [`CommitId`] for the synthetic commit that captures the inversion.
    async fn undo(&self, workspace: WorkspaceId, op_id: OperationId) -> Result<CommitId>;

    /// List paths beneath `prefix` (directory-style listing).
    async fn list(&self, workspace: WorkspaceId, prefix: &StorePath) -> Result<Vec<StorePath>>;
}
