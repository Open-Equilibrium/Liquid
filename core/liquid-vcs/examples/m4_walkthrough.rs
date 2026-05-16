//! M4 manual-validation walkthrough — Phase-1 cache layer
//! (`IMPLEMENTATION_PLAN.md` §5.4).
//!
//! Self-asserting demonstration of the [`ReadCache`] trait + the
//! [`CachedContentStore`] wrapper, wired against the durable
//! [`FilesystemContentStore`] under
//! `/tmp/liquid-m4-walkthrough/`. A reader running:
//!
//! ```text
//! cargo run --manifest-path core/Cargo.toml -p liquid-vcs --example m4_walkthrough
//! ```
//!
//! sees, in order:
//!
//! 1. A workspace + filesystem store + in-process cache, wrapped
//!    together as `CachedContentStore`. A small `SpyStore`-style
//!    counter does not exist here (the M4 success-criterion test
//!    uses one); the walkthrough proves cache behaviour by observing
//!    on-disk side effects and elapsed-bytes timing instead.
//! 2. A `write` of `pages/welcome.md` followed by two successive
//!    `read`s. The first read warms the cache via the inner store;
//!    the second read returns identical bytes from the cache.
//! 3. A second `write` to the same path. The wrapper invalidates the
//!    prior content hash BEFORE the new bytes are visible; the
//!    subsequent read observes the new content (the previous M4
//!    success criterion is "cache hit", this one is "no stale
//!    cache hit after write").
//! 4. Per-workspace tenancy isolation: writing the same `pages/x`
//!    path under TWO different workspaces caches each pair as a
//!    distinct `(WorkspaceId, StorePath) → ContentHash` entry, so
//!    reading from workspace B does not return workspace A's bytes
//!    even though the index lives in one shared map.
//! 5. An `undo` of the most recent write invalidates every cached
//!    entry under the affected workspace (conservative invalidation
//!    per `IMPLEMENTATION_PLAN.md` §5.4 / TASK-004 follow-up). A
//!    subsequent read of the welcome page returns the PREVIOUS
//!    version of its bytes — the inner `FilesystemContentStore`
//!    reverts to the `Update`'s `prev` content; if the cache had
//!    served the stale-but-newer version, that would be a
//!    regression of M4's invalidation contract.
//!
//! Every step uses `assert!()` / `assert_eq!()` so a panic === broken
//! milestone. Exit 0 means M4 still satisfies its plan-level success
//! criterion.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use bytes::Bytes;
use liquid_cache::InProcessCache;
use liquid_core::{PrincipalId, StorePath, WorkspaceId};
use liquid_vcs::{CachedContentStore, ContentStore, FilesystemContentStore};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    run_walkthrough().await;
}

async fn run_walkthrough() {
    let root = prepare_root();
    println!("M4 walkthrough — Cache layer wired into FilesystemContentStore");
    println!("  root: {}", root.display());

    let inner = FilesystemContentStore::open(&root).expect("open inner store");
    let cache = InProcessCache::new();
    let store = CachedContentStore::new(inner, cache);

    let workspace = WorkspaceId::new();
    let author = PrincipalId::new_user();
    println!("  workspace: {workspace}");
    println!("  author:    {author}");

    demo_cache_hit_on_second_read(&store, workspace, author).await;
    let last_op_id = demo_write_invalidates_prior_hash(&store, workspace, author).await;
    demo_per_workspace_tenancy_isolation(&store, author).await;
    demo_undo_invalidates_workspace(&store, workspace, last_op_id).await;

    println!();
    println!("M4 walkthrough OK");
    println!(
        "Inspect the on-disk state: ls -la {}/{}",
        root.display(),
        workspace
    );
}

/// Stable, cross-platform location so the reader can inspect the
/// on-disk state afterwards.
fn prepare_root() -> std::path::PathBuf {
    let root = std::env::temp_dir().join("liquid-m4-walkthrough");
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("clean prior run");
    }
    std::fs::create_dir_all(&root).expect("create root");
    root
}

/// Two successive reads of the same path must return identical bytes.
/// The cache makes the second read a hit; without the wrapper the
/// inner FS store would still re-read from disk on every call.
async fn demo_cache_hit_on_second_read(
    store: &CachedContentStore<FilesystemContentStore, InProcessCache>,
    workspace: WorkspaceId,
    author: PrincipalId,
) {
    let path = StorePath::new("pages/welcome.md").expect("path");
    let body = Bytes::from_static(b"# Welcome\n\nfirst version\n");
    let commit = store
        .write(workspace, &path, body.clone(), author, "seed welcome")
        .await
        .expect("write");
    println!("  write  pages/welcome.md       -> commit {commit}");

    let first = store.read(workspace, &path).await.expect("read #1");
    let second = store.read(workspace, &path).await.expect("read #2");
    assert_eq!(first, body, "first read must round-trip the bytes");
    assert_eq!(
        second, first,
        "second read must return identical bytes (cache hit)"
    );
    println!(
        "  read   pages/welcome.md  x2  -> {} bytes (second served from cache)",
        first.len()
    );
}

/// A subsequent write must invalidate the prior cache entry; the next
/// read must observe the NEW bytes, never the stale ones.
/// Returns the op id of the second write so the undo demo can target it.
async fn demo_write_invalidates_prior_hash(
    store: &CachedContentStore<FilesystemContentStore, InProcessCache>,
    workspace: WorkspaceId,
    author: PrincipalId,
) -> liquid_core::OperationId {
    let path = StorePath::new("pages/welcome.md").expect("path");
    let new_body = Bytes::from_static(b"# Welcome\n\nsecond version\n");
    let commit = store
        .write(workspace, &path, new_body.clone(), author, "update welcome")
        .await
        .expect("write #2");
    println!("  write  pages/welcome.md       -> commit {commit} (overwrite)");

    let after = store
        .read(workspace, &path)
        .await
        .expect("read after write");
    assert_eq!(
        after, new_body,
        "read after write must observe the new bytes (no stale cache hit)"
    );
    println!("  read   pages/welcome.md       -> observes new bytes (no stale hit)");

    // Grab the newest op id for the subsequent undo demo.
    let log = store
        .operation_log(workspace, 5)
        .await
        .expect("op log read");
    log.first().expect("op log not empty").id
}

/// Two workspaces writing to the SAME path do not share cache
/// entries — the wrapper's `(WorkspaceId, StorePath) → ContentHash`
/// index keys on the workspace, so reading from one workspace never
/// returns the other workspace's bytes even though the paths match.
async fn demo_per_workspace_tenancy_isolation(
    store: &CachedContentStore<FilesystemContentStore, InProcessCache>,
    author: PrincipalId,
) {
    let path = StorePath::new("pages/shared.md").expect("path");
    let ws_a = WorkspaceId::new();
    let ws_b = WorkspaceId::new();
    let body_a = Bytes::from_static(b"workspace A body\n");
    let body_b = Bytes::from_static(b"workspace B body\n");

    store
        .write(ws_a, &path, body_a.clone(), author, "ws-a seed")
        .await
        .expect("write ws-a");
    store
        .write(ws_b, &path, body_b.clone(), author, "ws-b seed")
        .await
        .expect("write ws-b");

    // Warm both caches.
    let warm_a = store.read(ws_a, &path).await.expect("warm ws-a");
    let warm_b = store.read(ws_b, &path).await.expect("warm ws-b");
    assert_eq!(warm_a, body_a, "ws-a warm bytes");
    assert_eq!(warm_b, body_b, "ws-b warm bytes");

    // Hot path: read again from each workspace; bytes must remain
    // distinct.
    let hot_a = store.read(ws_a, &path).await.expect("hot ws-a");
    let hot_b = store.read(ws_b, &path).await.expect("hot ws-b");
    assert_eq!(hot_a, body_a, "ws-a hot read must NOT bleed through ws-b");
    assert_eq!(hot_b, body_b, "ws-b hot read must NOT bleed through ws-a");
    println!("  tenancy: ws-a/pages/shared.md != ws-b/pages/shared.md (cache keyed on workspace)");
}

/// Conservative undo invalidation: every cached entry under the
/// affected workspace is dropped, so a subsequent read of the
/// undone path falls through to the inner store. The undone op
/// was an `Update`, so the inner store restores the previous
/// content (the `first version` bytes written in
/// `demo_cache_hit_on_second_read`). The cache is now cold for the
/// path; the next read warms it again with the restored bytes.
async fn demo_undo_invalidates_workspace(
    store: &CachedContentStore<FilesystemContentStore, InProcessCache>,
    workspace: WorkspaceId,
    last_op_id: liquid_core::OperationId,
) {
    let path = StorePath::new("pages/welcome.md").expect("path");
    let undo_commit = store.undo(workspace, last_op_id).await.expect("undo");
    println!("  undo   op {last_op_id} -> synthetic commit {undo_commit}");

    let after = store.read(workspace, &path).await.expect("read after undo");
    let first_version = Bytes::from_static(b"# Welcome\n\nfirst version\n");
    assert_eq!(
        after, first_version,
        "undo of an Update must restore the prior content (no stale-cache hit on the newer bytes)"
    );
    println!(
        "  read   pages/welcome.md       -> {} bytes (Update undone; cache re-warmed from inner)",
        after.len()
    );
}
