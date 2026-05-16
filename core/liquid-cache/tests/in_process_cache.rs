//! Integration tests for [`ReadCache`] / [`InProcessCache`] — M4 success
//! criterion (`IMPLEMENTATION_PLAN.md` §5.4).
//!
//! Each test asserts a specific behaviour from §4.3 (trait shape) or §9
//! (`liquid-cache` reference). The wired-into-`ContentStore` half of the
//! milestone lives in `core/liquid-vcs/tests/cached_store.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use bytes::Bytes;
use liquid_cache::{InProcessCache, ReadCache};
use liquid_core::ContentHash;

fn hash(seed: u8) -> ContentHash {
    // Reproducible test hash — 64 hex chars, lowercase, derived from a
    // single seed byte so each test can produce a stable distinct key
    // without pulling in `sha2`.
    let pattern = format!("{seed:02x}");
    let hex = pattern.repeat(32);
    ContentHash::from_hex(hex).expect("valid hex")
}

#[tokio::test]
async fn get_on_empty_cache_returns_none() {
    let cache = InProcessCache::new();
    assert!(cache.get(hash(0xa1)).await.is_none());
}

#[tokio::test]
async fn put_then_get_returns_the_stored_bytes() {
    let cache = InProcessCache::new();
    let key = hash(0xb2);
    let body = Bytes::from_static(b"hello cache");
    cache.put(key.clone(), body.clone()).await;
    let got = cache.get(key).await.expect("hit");
    assert_eq!(got, body);
}

#[tokio::test]
async fn put_overwrites_existing_entry_for_same_key() {
    let cache = InProcessCache::new();
    let key = hash(0xc3);
    cache.put(key.clone(), Bytes::from_static(b"old")).await;
    cache.put(key.clone(), Bytes::from_static(b"new")).await;
    let got = cache.get(key).await.expect("hit");
    assert_eq!(got.as_ref(), b"new");
}

#[tokio::test]
async fn invalidate_removes_entry() {
    let cache = InProcessCache::new();
    let key = hash(0xd4);
    cache.put(key.clone(), Bytes::from_static(b"present")).await;
    cache.invalidate(key.clone()).await;
    assert!(cache.get(key).await.is_none());
}

#[tokio::test]
async fn invalidate_missing_key_is_a_no_op() {
    let cache = InProcessCache::new();
    cache.invalidate(hash(0xe5)).await; // must not panic
}

#[tokio::test]
async fn distinct_keys_do_not_collide() {
    let cache = InProcessCache::new();
    let a = hash(0xa0);
    let b = hash(0xb0);
    cache.put(a.clone(), Bytes::from_static(b"alpha")).await;
    cache.put(b.clone(), Bytes::from_static(b"beta")).await;
    assert_eq!(cache.get(a).await.expect("a").as_ref(), b"alpha");
    assert_eq!(cache.get(b).await.expect("b").as_ref(), b"beta");
}

#[tokio::test]
async fn cache_is_clone_cheaply_and_shares_state() {
    // The Phase-1 design (§9) is `Arc<DashMap<…>>`; cloning the cache
    // handle must produce an additional reference into the SAME map so
    // the bridge layer can hand identical caches to every worker.
    let cache = InProcessCache::new();
    let handle = cache.clone();
    let key = hash(0xf6);
    cache.put(key.clone(), Bytes::from_static(b"shared")).await;
    assert_eq!(
        handle.get(key).await.expect("shared hit").as_ref(),
        b"shared"
    );
}

#[tokio::test]
async fn ready_for_dyn_dispatch_via_trait_object() {
    // Bridge / VCS layers consume a `dyn ReadCache` so the same callsite
    // can switch between InProcessCache (Phase 1) and RedisCache (Phase 3).
    // If this test fails to compile, `ReadCache` lost its dyn-safe
    // properties — see `IMPLEMENTATION_PLAN.md` §4.3.
    let cache: std::sync::Arc<dyn ReadCache> = std::sync::Arc::new(InProcessCache::new());
    let key = hash(0x07);
    cache.put(key.clone(), Bytes::from_static(b"dyn")).await;
    assert_eq!(cache.get(key).await.expect("dyn hit").as_ref(), b"dyn");
}
