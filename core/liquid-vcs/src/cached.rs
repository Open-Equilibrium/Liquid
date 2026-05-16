use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use async_trait::async_trait;
use bytes::Bytes;
use liquid_cache::ReadCache;
use liquid_core::{
    CommitId, ContentHash, OperationId, PrincipalId, Result, StorePath, WorkspaceId,
};

use crate::content_store::ContentStore;
use crate::operation::Operation;

type IndexMap = HashMap<(WorkspaceId, StorePath), ContentHash>;

/// `ContentStore` wrapper that warms a [`ReadCache`] on every `read`
/// hit and invalidates the prior content hash on every `write` /
/// workspace-invalidating call (`undo`).
///
/// Wired per `IMPLEMENTATION_PLAN.md` §5.4 / §9:
///
/// - `read(ws, path)` checks the wrapper's `(ws, path) → ContentHash`
///   index. On hit it asks the cache for the bytes; on miss it
///   delegates to the inner store, hashes the bytes, indexes
///   `(ws, path) → hash`, and warms the cache via `put`. Errors (e.g.
///   `NotFound`) are NEVER cached.
/// - `write(ws, path, content, ...)` invalidates the prior hash
///   recorded for `(ws, path)` BEFORE the new bytes are visible,
///   delegates to the inner store, then drops the index entry so the
///   next read re-hashes and re-warms (the write-side intentionally
///   does NOT warm; the M4 contract only specifies read-side
///   warming, so we avoid putting bytes the caller might never read).
/// - `undo(ws, _)` conservatively drops every index entry for
///   `workspace` and `invalidate`s the matching cached hashes. The
///   undo log itself does not yet expose which path was reverted; the
///   over-invalidation is safe and bounded by the per-workspace
///   index.
/// - `list` and `operation_log` pass through unchanged.
///
/// The wrapper is parametric over both the inner store and the cache
/// so callers can compose any pair, including dyn-dispatched ones via
/// `Arc<dyn ContentStore>` and `Arc<dyn ReadCache>` per §4.3.
///
/// Per `CLAUDE.md` Absolute Rule 5, every method takes a
/// `WorkspaceId` — the wrapper does not add a global namespace.
pub struct CachedContentStore<S, C> {
    inner: S,
    cache: C,
    /// `(workspace, path) → cached content hash`. Kept in-memory only
    /// (Phase 1 has no persisted index requirement); rebuilt lazily on
    /// the next read miss. The Mutex is uncontended in the common
    /// case (single bridge worker) and bounded in size by the number
    /// of live paths the runtime has read since startup.
    index: Mutex<IndexMap>,
}

impl<S, C> CachedContentStore<S, C>
where
    S: ContentStore,
    C: ReadCache,
{
    /// Wrap `inner` and `cache`. Both are owned — callers wanting
    /// shared ownership pass `Arc<...>`s (`Arc<S>` already implements
    /// `ContentStore` via the `async_trait` blanket).
    pub fn new(inner: S, cache: C) -> Self {
        Self {
            inner,
            cache,
            index: Mutex::new(HashMap::new()),
        }
    }

    /// Acquire the index lock, transparently recovering from poison.
    /// If a previous holder of the lock panicked, the inner `HashMap`
    /// may be in a stale-but-not-unsafe state — at worst the cache
    /// returns an out-of-date hash, which the cache layer already
    /// handles by re-reading from the inner store. We therefore
    /// silently take the inner data via `PoisonError::into_inner`
    /// rather than propagating an error every caller would have to
    /// handle. Using `unwrap_or_else` keeps us Absolute-Rule-1
    /// compliant (the rule forbids `.unwrap()` / `.expect()` only).
    fn lock_index(&self) -> MutexGuard<'_, IndexMap> {
        self.index
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[async_trait]
impl<S, C> ContentStore for CachedContentStore<S, C>
where
    S: ContentStore,
    C: ReadCache,
{
    async fn read(&self, workspace: WorkspaceId, path: &StorePath) -> Result<Bytes> {
        // Look up the cached hash for (ws, path). If we have one and
        // the cache still holds the bytes, return them without
        // touching the inner store — this is the M4 success-criterion
        // path.
        let cached_hash = self.lock_index().get(&(workspace, path.clone())).cloned();
        if let Some(hash) = cached_hash {
            if let Some(bytes) = self.cache.get(hash).await {
                return Ok(bytes);
            }
            // Stale index entry — the cache evicted the hash. Fall
            // through to the inner read; we'll re-hash and re-index.
        }

        let bytes = self.inner.read(workspace, path).await?;
        let new_hash = ContentHash::of_bytes(&bytes);
        self.lock_index()
            .insert((workspace, path.clone()), new_hash.clone());
        self.cache.put(new_hash, bytes.clone()).await;
        Ok(bytes)
    }

    async fn write(
        &self,
        workspace: WorkspaceId,
        path: &StorePath,
        content: Bytes,
        author: PrincipalId,
        message: &str,
    ) -> Result<CommitId> {
        // Invalidate the prior hash so a stale entry can never serve
        // post-write reads. The index entry itself is also dropped;
        // the next read re-hashes the (now-new) bytes and re-warms.
        //
        // Phase-1 known limitation: if the inner write returns Err,
        // the cache is already invalidated even though the inner
        // store still holds the OLD bytes. Correctness is preserved
        // (the next read falls through and re-warms), but the warm
        // cache entry is lost — a silent performance regression on
        // failure. Phase 3 (when retry semantics arrive on the
        // bridge layer) should snapshot-then-rollback the index
        // mutation on inner-call failure. Tracked under M4 follow-up.
        let prior = self.lock_index().remove(&(workspace, path.clone()));
        if let Some(hash) = prior {
            self.cache.invalidate(hash).await;
        }
        self.inner
            .write(workspace, path, content, author, message)
            .await
    }

    async fn operation_log(&self, workspace: WorkspaceId, limit: usize) -> Result<Vec<Operation>> {
        self.inner.operation_log(workspace, limit).await
    }

    async fn undo(&self, workspace: WorkspaceId, op_id: OperationId) -> Result<CommitId> {
        // Conservative: invalidate every cached hash for the
        // workspace, then forward. Phase-3 will revisit when the
        // operation log carries enough information to do a precise
        // path-targeted invalidation (TASK-004's jj-lib backend
        // exposes per-op affected-paths).
        //
        // Phase-1 known limitation (same shape as `write` above): if
        // the inner undo returns Err, the cache is already flushed
        // and correctness is preserved but a warm slice of the
        // cache is lost. Same Phase-3 follow-up applies.
        let drained: Vec<ContentHash> = self
            .lock_index()
            .extract_if(|(ws, _), _| *ws == workspace)
            .map(|(_, hash)| hash)
            .collect();
        for hash in drained {
            self.cache.invalidate(hash).await;
        }
        self.inner.undo(workspace, op_id).await
    }

    async fn list(&self, workspace: WorkspaceId, prefix: &StorePath) -> Result<Vec<StorePath>> {
        self.inner.list(workspace, prefix).await
    }
}
