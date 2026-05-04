//! Integration tests for `InMemoryContentStore`.
//!
//! Includes the `IMPLEMENTATION_PLAN.md` §5.2 success criterion: create a
//! workspace, write three files, read them back, undo the last write, verify
//! the file is gone.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use bytes::Bytes;
use liquid_core::{LiquidError, PrincipalId, StorePath, WorkspaceId};
use liquid_vcs::{ContentStore, InMemoryContentStore, OperationKind};

fn p(s: &str) -> StorePath {
    StorePath::new(s).expect("valid path")
}

#[tokio::test]
async fn write_then_read_returns_same_bytes() {
    let store = InMemoryContentStore::new();
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
async fn read_unknown_path_is_not_found() {
    let store = InMemoryContentStore::new();
    let ws = WorkspaceId::new();
    let err = store.read(ws, &p("nope.txt")).await.unwrap_err();
    assert!(matches!(err, LiquidError::NotFound(_)), "got {err:?}");
}

#[tokio::test]
async fn read_unknown_workspace_is_not_found() {
    let store = InMemoryContentStore::new();
    let err = store
        .read(WorkspaceId::new(), &p("a.txt"))
        .await
        .unwrap_err();
    assert!(matches!(err, LiquidError::NotFound(_)));
}

#[tokio::test]
async fn op_log_records_writes_newest_first() {
    let store = InMemoryContentStore::new();
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
async fn op_log_respects_limit() {
    let store = InMemoryContentStore::new();
    let ws = WorkspaceId::new();
    let author = PrincipalId::new_user();
    for i in 0..5 {
        store
            .write(
                ws,
                &p(&format!("f{i}")),
                Bytes::from_static(b"x"),
                author,
                "msg",
            )
            .await
            .expect("w");
    }
    let log = store.operation_log(ws, 3).await.expect("log");
    assert_eq!(log.len(), 3);
}

#[tokio::test]
async fn op_log_kinds_distinguish_create_and_update() {
    let store = InMemoryContentStore::new();
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
    // newest first: log[0] = update, log[1] = create
    assert!(matches!(log[0].kind, OperationKind::Update { .. }));
    assert!(matches!(log[1].kind, OperationKind::Create { .. }));
}

#[tokio::test]
async fn undo_create_removes_path() {
    let store = InMemoryContentStore::new();
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
    let store = InMemoryContentStore::new();
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
    let store = InMemoryContentStore::new();
    let ws = WorkspaceId::new();
    let author = PrincipalId::new_user();
    store
        .write(ws, &p("a"), Bytes::from_static(b"x"), author, "create")
        .await
        .expect("w");
    let err = store
        .undo(ws, liquid_core::OperationId::new())
        .await
        .unwrap_err();
    assert!(matches!(err, LiquidError::NotFound(_)));
}

#[tokio::test]
async fn list_returns_paths_under_prefix() {
    let store = InMemoryContentStore::new();
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
async fn workspaces_are_isolated() {
    let store = InMemoryContentStore::new();
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

/// Plan-level M2 success criterion (`IMPLEMENTATION_PLAN.md` §5.2).
#[tokio::test]
async fn plan_m2_success_criterion() {
    let store = InMemoryContentStore::new();
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

    assert_eq!(
        store.read(ws, &p("a")).await.expect("ra"),
        Bytes::from_static(b"A")
    );
    assert_eq!(
        store.read(ws, &p("b")).await.expect("rb"),
        Bytes::from_static(b"B")
    );
    assert_eq!(
        store.read(ws, &p("c")).await.expect("rc"),
        Bytes::from_static(b"C")
    );

    let log = store.operation_log(ws, 1).await.expect("log");
    let last = log[0].id;
    store.undo(ws, last).await.expect("undo");

    let err = store.read(ws, &p("c")).await.unwrap_err();
    assert!(
        matches!(err, LiquidError::NotFound(_)),
        "after undo of last write, path c must be gone — got {err:?}"
    );
    // a and b remain
    assert!(store.read(ws, &p("a")).await.is_ok());
    assert!(store.read(ws, &p("b")).await.is_ok());
}
