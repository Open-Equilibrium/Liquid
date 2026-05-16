//! M2 manual-validation walkthrough — Phase-1 VCS layer
//! (`IMPLEMENTATION_PLAN.md` §5.2).
//!
//! Self-asserting demonstration of the [`ContentStore`] trait and the
//! [`FilesystemContentStore`] backend. A reader running:
//!
//! ```text
//! cargo run --manifest-path core/Cargo.toml -p liquid-vcs --example m2_walkthrough
//! ```
//!
//! sees, in order:
//!
//! 1. A workspace + filesystem store rooted at `/tmp/liquid-m2-walkthrough/`
//!    (cleaned at start, kept after the run so it can be inspected).
//! 2. Three `write` calls — `pages/welcome.md`, `pages/notes.md`,
//!    `pages/todo.md` — each returning a fresh `CommitId`.
//! 3. A `read` of each path, asserting the bytes round-trip exactly.
//! 4. A `list` under `pages/` listing all three files.
//! 5. An `operation_log` of size 3, newest first.
//! 6. An `undo` of the most recent write, followed by a `read` that
//!    must now error with `LiquidError::NotFound`.
//! 7. A final byte-level peek at the on-disk layout
//!    (`files/<store_path>` + `op_log.jsonl`).
//!
//! Every step uses `assert!()` / `assert_eq!()` so a panic === broken
//! milestone. Exit 0 means M2 still satisfies its plan-level success
//! criterion.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use bytes::Bytes;
use liquid_core::{LiquidError, OperationId, PrincipalId, StorePath, WorkspaceId};
use liquid_vcs::{ContentStore, FilesystemContentStore, OperationKind};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    run_walkthrough().await;
}

const SEED_PAGES: [(&str, &str); 3] = [
    (
        "pages/welcome.md",
        "# Welcome\n\nThis is the welcome page.\n",
    ),
    ("pages/notes.md", "- buy milk\n- review PR\n"),
    (
        "pages/todo.md",
        "1. ship M2 walkthrough\n2. ship M3 walkthrough\n",
    ),
];

/// Top-level orchestrator. Split out of `main` so each phase of the
/// walkthrough is a single, named function — easier to read, easier
/// to keep under clippy's `too_many_lines` ceiling without an
/// `#[allow]` escape hatch.
async fn run_walkthrough() {
    let root = prepare_root();
    println!("M2 walkthrough — Filesystem ContentStore");
    println!("  root: {}", root.display());

    let store = FilesystemContentStore::open(&root).expect("open store");
    let workspace = WorkspaceId::new();
    let author = PrincipalId::new_user();
    println!("  workspace: {workspace}");
    println!("  author: {author}");

    seed_writes(&store, workspace, author).await;
    verify_round_trip_reads(&store, workspace).await;
    verify_list(&store, workspace).await;
    let last_op_id = inspect_op_log(&store, workspace).await;
    verify_undo(&store, workspace, last_op_id).await;
    verify_on_disk_layout(&root, workspace);
}

/// Stable, cross-platform location so the reader can inspect the
/// on-disk state afterwards. `temp_dir()` is `/tmp` on Linux, the OS
/// temp dir on macOS/Windows.
fn prepare_root() -> std::path::PathBuf {
    let root = std::env::temp_dir().join("liquid-m2-walkthrough");
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("clean prior run");
    }
    std::fs::create_dir_all(&root).expect("create root");
    root
}

async fn seed_writes(store: &FilesystemContentStore, workspace: WorkspaceId, author: PrincipalId) {
    for (raw, body) in SEED_PAGES {
        let path = StorePath::new(raw).expect("path");
        let commit = store
            .write(
                workspace,
                &path,
                Bytes::from(body.as_bytes().to_vec()),
                author,
                &format!("seed {raw}"),
            )
            .await
            .expect("write");
        println!("  write  {raw:<20} -> commit {commit}");
    }
}

async fn verify_round_trip_reads(store: &FilesystemContentStore, workspace: WorkspaceId) {
    for (raw, body) in SEED_PAGES {
        let path = StorePath::new(raw).expect("path");
        let got = store.read(workspace, &path).await.expect("read");
        assert_eq!(
            got.as_ref(),
            body.as_bytes(),
            "round-trip mismatch for {raw}"
        );
        println!("  read   {raw:<20} -> {} bytes (OK)", got.len());
    }
}

async fn verify_list(store: &FilesystemContentStore, workspace: WorkspaceId) {
    let prefix = StorePath::new("pages").expect("prefix");
    let mut listed = store.list(workspace, &prefix).await.expect("list");
    listed.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    assert_eq!(listed.len(), 3, "list should return 3 paths");
    for p in &listed {
        println!("  list   {}", p.as_str());
    }
}

/// Returns the most recent op id — the caller threads it into `undo`.
async fn inspect_op_log(store: &FilesystemContentStore, workspace: WorkspaceId) -> OperationId {
    let log = store.operation_log(workspace, 10).await.expect("op log");
    assert_eq!(log.len(), 3, "op log should have 3 entries");
    let newest = log.first().expect("at least one op");
    let newest_path = match &newest.kind {
        OperationKind::Create { path, .. }
        | OperationKind::Update { path, .. }
        | OperationKind::Delete { path, .. } => path.as_str(),
        OperationKind::Undo { .. } => "<undo>",
    };
    println!(
        "  op-log size={} newest_op={} newest_path={}",
        log.len(),
        newest.id,
        newest_path
    );
    newest.id
}

async fn verify_undo(
    store: &FilesystemContentStore,
    workspace: WorkspaceId,
    last_op_id: OperationId,
) {
    let undo_commit = store.undo(workspace, last_op_id).await.expect("undo");
    println!("  undo   op {last_op_id} -> synthetic commit {undo_commit}");
    // The newest op corresponds to the last write — `SEED_PAGES[2]`,
    // `pages/todo.md`. Changing the SEED_PAGES order would invalidate
    // this assertion; the assertion is the canary.
    let undone_path = StorePath::new("pages/todo.md").expect("path");
    let err = store
        .read(workspace, &undone_path)
        .await
        .expect_err("undone path should be gone");
    assert!(
        matches!(err, LiquidError::NotFound { .. }),
        "expected NotFound, got {err:?}"
    );
    println!("  read   pages/todo.md       -> NotFound (as expected)");
}

fn verify_on_disk_layout(root: &std::path::Path, workspace: WorkspaceId) {
    let ws_root = root.join(workspace.to_string());
    let files_dir = ws_root.join("files");
    let op_log = ws_root.join("op_log.jsonl");
    assert!(files_dir.is_dir(), "missing {}", files_dir.display());
    assert!(op_log.is_file(), "missing {}", op_log.display());
    let op_log_lines = std::fs::read_to_string(&op_log)
        .expect("op log")
        .lines()
        .count();
    println!(
        "  layout files_dir={} op_log lines={}",
        files_dir.display(),
        op_log_lines
    );

    println!();
    println!("M2 walkthrough OK");
    println!(
        "Inspect the on-disk state: ls -la {} && cat {}",
        ws_root.display(),
        op_log.display()
    );
}
