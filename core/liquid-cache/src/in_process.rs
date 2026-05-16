use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use dashmap::DashMap;
use liquid_core::ContentHash;

use crate::read_cache::ReadCache;

/// Phase-1 in-process implementation of [`ReadCache`].
///
/// Stores entries in an `Arc<DashMap<ContentHash, Bytes>>` so the
/// handle is cheap to clone and every clone shares the same backing
/// map. No expiry, no size cap — Phase 3 ADR will revisit when
/// `RedisCache` lands and the two backends need to agree on an
/// eviction policy.
#[derive(Debug, Clone, Default)]
pub struct InProcessCache {
    inner: Arc<DashMap<ContentHash, Bytes>>,
}

impl InProcessCache {
    /// Build an empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of currently-cached entries. Useful for tests and the
    /// `cache stats` admin path; not part of the trait.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// `true` iff no entries are cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[async_trait]
impl ReadCache for InProcessCache {
    async fn get(&self, key: ContentHash) -> Option<Bytes> {
        self.inner.get(&key).map(|v| v.value().clone())
    }

    async fn put(&self, key: ContentHash, value: Bytes) {
        self.inner.insert(key, value);
    }

    async fn invalidate(&self, key: ContentHash) {
        self.inner.remove(&key);
    }
}
