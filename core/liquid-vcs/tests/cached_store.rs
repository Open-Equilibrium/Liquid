//! M4 success criterion — wired-in `ReadCache` via
//! [`CachedContentStore`].
//!
//! Per `IMPLEMENTATION_PLAN.md` §5.4: "Second read of the same content
//! hits the cache (verified by spying on the mock `ContentStore` — the
//! second call must not reach Jujutsu)."
//!
//! These tests use a tiny spy `ContentStore` (`SpyStore`) that counts
//! every method call. The cache wrapper must let the second `read` of
//! the same path-and-content avoid the inner store, and must
//! `invalidate` the previously-cached hash before a `write` returns.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use bytes::Bytes;
use liquid_cache::InProcessCache;
use liquid_core::{
    CommitId, LiquidError, OperationId, PrincipalId, Result, StorePath, WorkspaceId,
};
use liquid_vcs::{CachedContentStore, ContentStore, Operation, OperationKind};

// ── Spy ContentStore ─────────────────────────────────────────────────
//
// Backed by an in-memory `HashMap<(WorkspaceId, StorePath), Bytes>`
// plus per-method call counters so the test can assert which calls
// were forwarded vs. served from the cache. Intentionally minimal —
// no operation log, no real undo — because the M4 contract is about
// the cache wiring, not the inner store's correctness (covered by
// the existing `liquid-vcs` integration tests).

#[derive(Default)]
struct SpyCounts {
    read: AtomicUsize,
    write: AtomicUsize,
    list: AtomicUsize,
    operation_log: AtomicUsize,
    undo: AtomicUsize,
}

#[derive(Clone)]
struct SpyStore {
    state: Arc<Mutex<HashMap<(WorkspaceId, StorePath), Bytes>>>,
    counts: Arc<SpyCounts>,
}

impl SpyStore {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            counts: Arc::new(SpyCounts::default()),
        }
    }

    fn counts(&self) -> Arc<SpyCounts> {
        self.counts.clone()
    }
}

#[async_trait]
impl ContentStore for SpyStore {
    async fn read(&self, workspace: WorkspaceId, path: &StorePath) -> Result<Bytes> {
        self.counts.read.fetch_add(1, Ordering::SeqCst);
        let map = self.state.lock().expect("spy state mutex");
        map.get(&(workspace, path.clone()))
            .cloned()
            .ok_or_else(|| LiquidError::NotFound(path.as_str().to_owned()))
    }

    async fn write(
        &self,
        workspace: WorkspaceId,
        path: &StorePath,
        content: Bytes,
        _author: PrincipalId,
        _message: &str,
    ) -> Result<CommitId> {
        self.counts.write.fetch_add(1, Ordering::SeqCst);
        let mut map = self.state.lock().expect("spy state mutex");
        map.insert((workspace, path.clone()), content);
        Ok(CommitId::new())
    }

    async fn operation_log(
        &self,
        _workspace: WorkspaceId,
        _limit: usize,
    ) -> Result<Vec<Operation>> {
        self.counts.operation_log.fetch_add(1, Ordering::SeqCst);
        Ok(Vec::new())
    }

    async fn undo(&self, workspace: WorkspaceId, _op_id: OperationId) -> Result<CommitId> {
        // Pretend undo: clear every entry for the workspace so a
        // subsequent read sees NotFound. Sufficient for the test —
        // the real undo lives in the in-memory/filesystem stores.
        self.counts.undo.fetch_add(1, Ordering::SeqCst);
        let mut map = self.state.lock().expect("spy state mutex");
        map.retain(|(ws, _), _| *ws != workspace);
        Ok(CommitId::new())
    }

    async fn list(&self, workspace: WorkspaceId, prefix: &StorePath) -> Result<Vec<StorePath>> {
        self.counts.list.fetch_add(1, Ordering::SeqCst);
        let map = self.state.lock().expect("spy state mutex");
        let prefix_str = prefix.as_str();
        let mut out: Vec<StorePath> = map
            .keys()
            .filter(|(ws, p)| *ws == workspace && p.as_str().starts_with(prefix_str))
            .map(|(_, p)| p.clone())
            .collect();
        out.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        Ok(out)
    }
}

fn fresh() -> (
    SpyStore,
    Arc<SpyCounts>,
    InProcessCache,
    CachedContentStore<SpyStore, InProcessCache>,
) {
    let spy = SpyStore::new();
    let counts = spy.counts();
    let cache = InProcessCache::new();
    let cached = CachedContentStore::new(spy.clone(), cache.clone());
    (spy, counts, cache, cached)
}

fn p(s: &str) -> StorePath {
    StorePath::new(s).expect("path")
}

// ── Tests ────────────────────────────────────────────────────────────

#[tokio::test]
async fn second_read_of_same_path_is_served_from_cache() {
    let (_spy, counts, cache, store) = fresh();
    let ws = WorkspaceId::new();
    let path = p("welcome.md");
    let author = PrincipalId::new_user();
    let body = Bytes::from_static(b"hello");

    // Seed via the cached wrapper so cache + spy state are coherent.
    store
        .write(ws, &path, body.clone(), author, "seed")
        .await
        .expect("write");
    assert_eq!(counts.write.load(Ordering::SeqCst), 1, "one inner write");

    // First read: cache miss (the write side intentionally does NOT
    // warm the cache — only reads do, per §5.4). Forwards to spy.
    let got = store.read(ws, &path).await.expect("first read");
    assert_eq!(got, body);
    assert_eq!(counts.read.load(Ordering::SeqCst), 1, "first read forwards");
    assert_eq!(cache.len(), 1, "first read warms the cache");

    // Second read: must hit the cache and NOT forward.
    let got2 = store.read(ws, &path).await.expect("second read");
    assert_eq!(got2, body);
    assert_eq!(
        counts.read.load(Ordering::SeqCst),
        1,
        "M4 success criterion: second read must NOT reach the inner store"
    );
}

#[tokio::test]
async fn write_invalidates_prior_hash_so_next_read_observes_new_content() {
    let (_spy, counts, _cache, store) = fresh();
    let ws = WorkspaceId::new();
    let path = p("doc.txt");
    let author = PrincipalId::new_user();

    store
        .write(ws, &path, Bytes::from_static(b"v1"), author, "v1")
        .await
        .expect("write v1");

    // Warm the cache by reading.
    let r1 = store.read(ws, &path).await.expect("read v1");
    assert_eq!(r1.as_ref(), b"v1");
    let reads_after_v1 = counts.read.load(Ordering::SeqCst);

    // Overwrite. The cache must invalidate the prior hash before the
    // new bytes are visible.
    store
        .write(ws, &path, Bytes::from_static(b"v2"), author, "v2")
        .await
        .expect("write v2");

    let r2 = store.read(ws, &path).await.expect("read v2");
    assert_eq!(r2.as_ref(), b"v2", "second read must observe v2, not v1");
    assert!(
        counts.read.load(Ordering::SeqCst) > reads_after_v1,
        "read after invalidation must reach the inner store"
    );
}

#[tokio::test]
async fn read_miss_propagates_inner_not_found_without_caching() {
    let (_spy, counts, cache, store) = fresh();
    let ws = WorkspaceId::new();
    let path = p("missing.md");

    let err = store
        .read(ws, &path)
        .await
        .expect_err("missing path should error");
    assert!(matches!(err, LiquidError::NotFound(_)));
    assert_eq!(counts.read.load(Ordering::SeqCst), 1);
    assert!(cache.is_empty(), "NotFound must not poison the cache");

    // Subsequent read should ALSO forward — error must not be cached.
    let _ = store.read(ws, &path).await.expect_err("still not found");
    assert_eq!(counts.read.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn distinct_paths_with_identical_bytes_share_one_cache_entry() {
    // Cache is content-addressable (§9): the entry is keyed by
    // SHA-256(bytes), so two paths storing identical bytes share one
    // cache slot. A subsequent read of either path is served from
    // that single slot.
    let (_spy, counts, cache, store) = fresh();
    let ws = WorkspaceId::new();
    let author = PrincipalId::new_user();
    let body = Bytes::from_static(b"identical");

    let a = p("a.md");
    let b = p("b.md");
    store
        .write(ws, &a, body.clone(), author, "a")
        .await
        .expect("w a");
    store
        .write(ws, &b, body.clone(), author, "b")
        .await
        .expect("w b");

    let _ = store.read(ws, &a).await.expect("r a");
    let _ = store.read(ws, &b).await.expect("r b");

    assert_eq!(
        cache.len(),
        1,
        "content-addressable cache must dedupe identical bytes"
    );
    assert_eq!(counts.read.load(Ordering::SeqCst), 2, "both reads forward");
}

#[tokio::test]
async fn undo_invalidates_cached_entries_for_the_workspace() {
    // After undo we cannot know which content hash was removed without
    // re-reading the op log, so the conservative correctness policy
    // for the Phase-1 stub is to invalidate every cached entry tied
    // to the workspace whose state just changed. A read after undo
    // therefore must reach the inner store.
    let (_spy, counts, _cache, store) = fresh();
    let ws = WorkspaceId::new();
    let path = p("note.md");
    let author = PrincipalId::new_user();

    store
        .write(ws, &path, Bytes::from_static(b"x"), author, "x")
        .await
        .expect("write");
    let _ = store.read(ws, &path).await.expect("read warms cache");
    let reads_before_undo = counts.read.load(Ordering::SeqCst);

    store.undo(ws, OperationId::new()).await.expect("undo");
    // SpyStore::undo clears state for the workspace; the read should
    // now reach the inner store (cache invalidated) and surface
    // NotFound from it.
    let err = store.read(ws, &path).await.expect_err("must miss");
    assert!(matches!(err, LiquidError::NotFound(_)));
    assert!(
        counts.read.load(Ordering::SeqCst) > reads_before_undo,
        "post-undo read must reach inner store, not serve stale cache"
    );
}

#[tokio::test]
async fn list_and_operation_log_pass_through_unchanged() {
    let (_spy, counts, _cache, store) = fresh();
    let ws = WorkspaceId::new();
    let author = PrincipalId::new_user();

    store
        .write(ws, &p("notes/a.md"), Bytes::from_static(b"a"), author, "a")
        .await
        .expect("write a");
    store
        .write(ws, &p("notes/b.md"), Bytes::from_static(b"b"), author, "b")
        .await
        .expect("write b");

    let listed = store.list(ws, &p("notes")).await.expect("list");
    assert_eq!(listed.len(), 2);
    assert_eq!(counts.list.load(Ordering::SeqCst), 1);

    let log = store.operation_log(ws, 10).await.expect("op log");
    assert!(log.is_empty(), "SpyStore returns empty op log");
    assert_eq!(counts.operation_log.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn cache_is_independent_per_workspace_at_key_level() {
    // Although the cache key is the content hash (shared across
    // workspaces), the path→hash index inside the wrapper MUST be
    // keyed by `(WorkspaceId, StorePath)`. Otherwise a read of
    // `pages/p` in workspace B would steal workspace A's hash and
    // return A's bytes — a tenancy isolation violation.
    let (_spy, counts, _cache, store) = fresh();
    let path = p("page.md");
    let author = PrincipalId::new_user();

    let ws_a = WorkspaceId::new();
    let ws_b = WorkspaceId::new();
    store
        .write(ws_a, &path, Bytes::from_static(b"A"), author, "A")
        .await
        .expect("write A");
    store
        .write(ws_b, &path, Bytes::from_static(b"B"), author, "B")
        .await
        .expect("write B");

    // Warm both.
    let _ = store.read(ws_a, &path).await.expect("r A");
    let _ = store.read(ws_b, &path).await.expect("r B");
    let reads_after_warm = counts.read.load(Ordering::SeqCst);

    // Second read of A must serve from cache and return A bytes, not B.
    let r_a = store.read(ws_a, &path).await.expect("r A again");
    assert_eq!(r_a.as_ref(), b"A");
    assert_eq!(
        counts.read.load(Ordering::SeqCst),
        reads_after_warm,
        "second read of workspace A must hit cache"
    );

    let r_b = store.read(ws_b, &path).await.expect("r B again");
    assert_eq!(r_b.as_ref(), b"B");
    assert_eq!(
        counts.read.load(Ordering::SeqCst),
        reads_after_warm,
        "second read of workspace B must also hit cache (proves both isolation and effectiveness)"
    );
}

// `Operation` and `OperationKind` are re-exported through the trait so
// the SpyStore can construct them in future tests. Touching here so
// the import does not become dead.
#[allow(dead_code)]
fn _types_in_scope(_: Operation, _: OperationKind) {}
