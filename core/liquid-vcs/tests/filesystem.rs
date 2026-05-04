//! Integration tests for `FilesystemContentStore`.
//!
//! Mirrors the in-memory suite where the contract is identical, plus
//! durability checks specific to the on-disk backend (write -> drop ->
//! reopen -> read still works).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use bytes::Bytes;
use liquid_core::{LiquidError, OperationId, PrincipalId, StorePath, WorkspaceId};
use liquid_vcs::{ContentStore, FilesystemContentStore, OperationKind};
use tempfile::TempDir;

fn p(s: &str) -> StorePath {
    StorePath::new(s).expect("valid path")
}

fn fresh() -> (TempDir, FilesystemContentStore) {
    let dir = TempDir::new().expect("tempdir");
    let store = FilesystemContentStore::open(dir.path()).expect("open store");
    (dir, store)
}

#[tokio::test]
async fn write_then_read_returns_same_bytes() {
    let (_dir, store) = fresh();
    let ws = WorkspaceId::new();
    let author = PrincipalId::new_user();

    store
        .write(
            ws,
            &p("a.txt"),
            Bytes::from_static(b"hello"),
            author,
            "first",
        )
        .await
        .expect("write");

    let got = store.read(ws, &p("a.txt")).await.expect("read");
    assert_eq!(got, Bytes::from_static(b"hello"));
}

#[tokio::test]
async fn read_unknown_workspace_is_not_found() {
    let (_dir, store) = fresh();
    let err = store
        .read(WorkspaceId::new(), &p("a.txt"))
        .await
        .unwrap_err();
    assert!(matches!(err, LiquidError::NotFound(_)));
}

#[tokio::test]
async fn read_unknown_path_in_existing_workspace_is_not_found() {
    let (_dir, store) = fresh();
    let ws = WorkspaceId::new();
    let author = PrincipalId::new_user();
    store
        .write(ws, &p("a"), Bytes::from_static(b"x"), author, "")
        .await
        .expect("w");
    let err = store.read(ws, &p("missing")).await.unwrap_err();
    assert!(matches!(err, LiquidError::NotFound(_)));
}

#[tokio::test]
async fn nested_store_paths_are_persisted() {
    let (_dir, store) = fresh();
    let ws = WorkspaceId::new();
    let author = PrincipalId::new_user();
    store
        .write(
            ws,
            &p("pages/sub/index.json"),
            Bytes::from_static(b"{}"),
            author,
            "",
        )
        .await
        .expect("w");
    let got = store.read(ws, &p("pages/sub/index.json")).await.expect("r");
    assert_eq!(got, Bytes::from_static(b"{}"));
}

#[tokio::test]
async fn op_log_records_writes_newest_first() {
    let (_dir, store) = fresh();
    let ws = WorkspaceId::new();
    let author = PrincipalId::new_user();
    store
        .write(ws, &p("one"), Bytes::from_static(b"1"), author, "m1")
        .await
        .expect("w1");
    store
        .write(ws, &p("two"), Bytes::from_static(b"2"), author, "m2")
        .await
        .expect("w2");
    let log = store.operation_log(ws, 10).await.expect("log");
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].message, "m2");
    assert_eq!(log[1].message, "m1");
}

#[tokio::test]
async fn op_log_kinds_distinguish_create_and_update() {
    let (_dir, store) = fresh();
    let ws = WorkspaceId::new();
    let author = PrincipalId::new_user();
    store
        .write(ws, &p("a"), Bytes::from_static(b"v1"), author, "create")
        .await
        .expect("w");
    store
        .write(ws, &p("a"), Bytes::from_static(b"v2"), author, "update")
        .await
        .expect("w");
    let log = store.operation_log(ws, 10).await.expect("log");
    assert!(matches!(log[0].kind, OperationKind::Update { .. }));
    assert!(matches!(log[1].kind, OperationKind::Create { .. }));
}

#[tokio::test]
async fn undo_create_removes_path_on_disk() {
    let (_dir, store) = fresh();
    let ws = WorkspaceId::new();
    let author = PrincipalId::new_user();
    store
        .write(ws, &p("a"), Bytes::from_static(b"x"), author, "create")
        .await
        .expect("w");
    let log = store.operation_log(ws, 1).await.expect("log");
    let target = log[0].id;
    store.undo(ws, target).await.expect("undo");

    let err = store.read(ws, &p("a")).await.unwrap_err();
    assert!(matches!(err, LiquidError::NotFound(_)));
}

#[tokio::test]
async fn undo_update_restores_previous_bytes() {
    let (_dir, store) = fresh();
    let ws = WorkspaceId::new();
    let author = PrincipalId::new_user();
    store
        .write(ws, &p("a"), Bytes::from_static(b"v1"), author, "create")
        .await
        .expect("w");
    store
        .write(ws, &p("a"), Bytes::from_static(b"v2"), author, "update")
        .await
        .expect("w");
    let log = store.operation_log(ws, 1).await.expect("log");
    let target = log[0].id;
    store.undo(ws, target).await.expect("undo");

    let got = store.read(ws, &p("a")).await.expect("read");
    assert_eq!(got, Bytes::from_static(b"v1"));
}

#[tokio::test]
async fn undo_unknown_operation_is_not_found() {
    let (_dir, store) = fresh();
    let ws = WorkspaceId::new();
    let author = PrincipalId::new_user();
    store
        .write(ws, &p("a"), Bytes::from_static(b"x"), author, "create")
        .await
        .expect("w");
    let err = store.undo(ws, OperationId::new()).await.unwrap_err();
    assert!(matches!(err, LiquidError::NotFound(_)));
}

#[tokio::test]
async fn list_returns_paths_under_prefix() {
    let (_dir, store) = fresh();
    let ws = WorkspaceId::new();
    let author = PrincipalId::new_user();
    for path in ["pages/a", "pages/b", "instances/x"] {
        store
            .write(ws, &p(path), Bytes::from_static(b"x"), author, "")
            .await
            .expect("w");
    }
    let mut got = store
        .list(ws, &p("pages"))
        .await
        .expect("list")
        .into_iter()
        .map(|sp| sp.as_str().to_owned())
        .collect::<Vec<_>>();
    got.sort();
    assert_eq!(got, vec!["pages/a".to_owned(), "pages/b".to_owned()]);
}

#[tokio::test]
async fn workspaces_are_isolated_on_disk() {
    let (_dir, store) = fresh();
    let a = WorkspaceId::new();
    let b = WorkspaceId::new();
    let author = PrincipalId::new_user();
    store
        .write(a, &p("file"), Bytes::from_static(b"a"), author, "")
        .await
        .expect("w");
    store
        .write(b, &p("file"), Bytes::from_static(b"b"), author, "")
        .await
        .expect("w");
    assert_eq!(
        store.read(a, &p("file")).await.expect("ra"),
        Bytes::from_static(b"a")
    );
    assert_eq!(
        store.read(b, &p("file")).await.expect("rb"),
        Bytes::from_static(b"b")
    );
}

/// Plan-level M2 success criterion (`IMPLEMENTATION_PLAN.md` §5.2), against
/// the on-disk backend.
#[tokio::test]
async fn plan_m2_success_criterion_on_disk() {
    let (_dir, store) = fresh();
    let ws = WorkspaceId::new();
    let author = PrincipalId::new_user();

    store
        .write(ws, &p("a"), Bytes::from_static(b"A"), author, "write a")
        .await
        .expect("w1");
    store
        .write(ws, &p("b"), Bytes::from_static(b"B"), author, "write b")
        .await
        .expect("w2");
    store
        .write(ws, &p("c"), Bytes::from_static(b"C"), author, "write c")
        .await
        .expect("w3");

    let log = store.operation_log(ws, 1).await.expect("log");
    let last = log[0].id;
    store.undo(ws, last).await.expect("undo");

    let err = store.read(ws, &p("c")).await.unwrap_err();
    assert!(matches!(err, LiquidError::NotFound(_)));
    assert!(store.read(ws, &p("a")).await.is_ok());
    assert!(store.read(ws, &p("b")).await.is_ok());
}

/// The point of a filesystem backend: state outlives the process. Drop the
/// store and reopen the same root; reads must succeed and the op log must
/// replay.
#[tokio::test]
async fn data_persists_across_store_instances() {
    let dir = TempDir::new().expect("tempdir");
    let ws = WorkspaceId::new();
    let author = PrincipalId::new_user();

    {
        let store = FilesystemContentStore::open(dir.path()).expect("open 1");
        store
            .write(ws, &p("a"), Bytes::from_static(b"persisted"), author, "m")
            .await
            .expect("write");
    } // store dropped here

    let store = FilesystemContentStore::open(dir.path()).expect("open 2");
    let got = store.read(ws, &p("a")).await.expect("read");
    assert_eq!(got, Bytes::from_static(b"persisted"));

    let log = store.operation_log(ws, 10).await.expect("log");
    assert_eq!(log.len(), 1);
    assert!(matches!(log[0].kind, OperationKind::Create { .. }));
}

/// After a process restart, undo must still be able to invert the op log
/// entry — proving the op log is replayed correctly off disk.
#[tokio::test]
async fn undo_works_after_reopening_store() {
    let dir = TempDir::new().expect("tempdir");
    let ws = WorkspaceId::new();
    let author = PrincipalId::new_user();

    let target = {
        let store = FilesystemContentStore::open(dir.path()).expect("open 1");
        store
            .write(ws, &p("a"), Bytes::from_static(b"v1"), author, "create")
            .await
            .expect("w");
        store
            .write(ws, &p("a"), Bytes::from_static(b"v2"), author, "update")
            .await
            .expect("w");
        let log = store.operation_log(ws, 1).await.expect("log");
        log[0].id
    };

    let store = FilesystemContentStore::open(dir.path()).expect("open 2");
    store.undo(ws, target).await.expect("undo");
    let got = store.read(ws, &p("a")).await.expect("read");
    assert_eq!(got, Bytes::from_static(b"v1"));
}
