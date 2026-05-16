use async_trait::async_trait;
use bytes::Bytes;
use liquid_core::ContentHash;

/// Content-addressable read cache (`IMPLEMENTATION_PLAN.md` §4.3).
///
/// All operations are async so the same trait covers in-process
/// (Phase 1) and out-of-process / network backends (Phase 3
/// `RedisCache`). Implementations must be `Send + Sync` so a single
/// instance can be shared across a tokio runtime.
#[async_trait]
pub trait ReadCache: Send + Sync {
    /// Return the bytes for `key` if cached, else `None`. A `None`
    /// result is not an error — the caller (typically
    /// `liquid-vcs::CachedContentStore`) falls through to the inner
    /// store, hashes the result, and warms the cache via [`Self::put`].
    async fn get(&self, key: ContentHash) -> Option<Bytes>;

    /// Insert / overwrite the entry for `key`. Idempotent. No TTL in
    /// the Phase-1 contract; eviction is the implementation's
    /// responsibility.
    async fn put(&self, key: ContentHash, value: Bytes);

    /// Exact invalidation — called by `CachedContentStore::write` on
    /// the old content hash before a successful write completes
    /// (`IMPLEMENTATION_PLAN.md` §5.4). Missing-key invalidation is a
    /// no-op so the call is always safe.
    async fn invalidate(&self, key: ContentHash);
}
