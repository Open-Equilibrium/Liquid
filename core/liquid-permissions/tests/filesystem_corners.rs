//! Coverage backfill for `liquid_permissions::FilesystemPermissionIndex`
//! — the happy-path suite in `tests/filesystem_index.rs` and the
//! end-to-end wiring in `tests/m3_end_to_end.rs` cover the main
//! `assign_role` / `check` matrix; this file fills in the open-time
//! discovery surface that those tests skip:
//!
//!   - `workspace_path` getter (used by diagnostics + `m3_walkthrough`)
//!   - re-open over an existing root with mixed-content children
//!     (a non-directory file, a non-UUID directory name, a workspace
//!     directory without `permissions.toml`)
//!   - re-open finds and preserves a previously-written workspace's
//!     bindings (atomic-write round-trip across two `open` calls)
//!
//! Mirrors the focused-single-assertion style of
//! `tests/m3_end_to_end.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;

use liquid_core::{Action, AppInstanceId, PrincipalId, Resource, WorkspaceId};
use liquid_permissions::{BuiltInRole, FilesystemPermissionIndex, PermissionIndex};
use tempfile::TempDir;

#[tokio::test]
async fn workspace_path_resolves_under_root() {
    let dir = TempDir::new().expect("tempdir");
    let idx = FilesystemPermissionIndex::open(dir.path()).expect("open");
    let workspace = WorkspaceId::new();
    let path = idx.workspace_path(workspace);
    assert!(path.starts_with(dir.path()));
    assert!(
        path.ends_with("permissions.toml"),
        "workspace_path must end at permissions.toml: {path:?}"
    );
    assert!(path.to_string_lossy().contains(&workspace.to_string()));
}

#[tokio::test]
async fn open_tolerates_stray_files_and_empty_workspace_dirs() {
    // Pre-seed `workspaces/` with two siblings the loader must walk
    // past without erroring:
    //
    //   1. a regular file (file_type != dir → `continue` at
    //      `filesystem.rs:62`)
    //   2. a UUID-named directory with no `permissions.toml` inside
    //      (path.exists() is false → `continue` at
    //      `filesystem.rs:67`)
    //
    // Both paths must be exercised so `open()` is robust against a
    // partially-initialised on-disk layout (e.g. a workspace that
    // was created but never had any role binding yet).
    let dir = TempDir::new().expect("tempdir");
    let ws_dir = dir.path().join("workspaces");
    fs::create_dir_all(&ws_dir).expect("workspaces dir");
    fs::write(ws_dir.join("README.md"), "not a workspace").expect("stray file");
    let uuid_named = WorkspaceId::new().to_string();
    fs::create_dir_all(ws_dir.join(&uuid_named)).expect("uuid-named dir without permissions.toml");

    let idx = FilesystemPermissionIndex::open(dir.path()).expect("open tolerates strays");

    // A workspace that never had bindings must still check cleanly
    // (returns Ok(false) — no panic, no error).
    let principal = PrincipalId::new_user();
    let app = AppInstanceId::new();
    let allowed = idx
        .check(principal, Action::Read, Resource::AppInstance(app))
        .await
        .expect("check on empty workspace");
    assert!(!allowed, "no role binding ⇒ check must return false");
}

#[tokio::test]
async fn open_rejects_non_uuid_workspace_directory_name() {
    // A directory under workspaces/ whose name is NOT a UUID is a
    // structural violation; `parse_workspace_id` must surface it as
    // `InvalidInput`, not a panic. The loader walks the directory
    // entries eagerly, so a single bad entry fails open() — which is
    // the conservative choice (refuse to come up with mis-keyed
    // state) over silently dropping bindings.
    let dir = TempDir::new().expect("tempdir");
    let ws_dir = dir.path().join("workspaces");
    fs::create_dir_all(ws_dir.join("not-a-uuid-at-all")).expect("bad dir");
    // Plant a real permissions.toml inside so the loader reaches the
    // UUID parse step rather than skipping for missing file.
    fs::write(
        ws_dir.join("not-a-uuid-at-all").join("permissions.toml"),
        "bindings = []\n",
    )
    .expect("plant file");

    let err = FilesystemPermissionIndex::open(dir.path()).expect_err("non-UUID dir name");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("not a UUID") || msg.contains("not-a-uuid-at-all"),
        "error must name the offending directory: {msg}"
    );
}

#[tokio::test]
async fn open_round_trips_bindings_across_two_open_calls() {
    let dir = TempDir::new().expect("tempdir");
    let workspace = WorkspaceId::new();
    let owner = PrincipalId::new_user();
    let app = AppInstanceId::new();

    // First instance: assign a role, drop without explicit flush.
    {
        let idx = FilesystemPermissionIndex::open(dir.path()).expect("open #1");
        idx.assign_role(workspace, owner, BuiltInRole::WorkspaceOwner, None)
            .await
            .expect("assign owner");
    }

    // Second instance over the same root must observe the binding.
    // (`PermissionIndex::check` is workspace-agnostic today — it
    // scans the full binding set — so we do not need to thread
    // `workspace` through the check call; the binding was assigned
    // under that workspace and persisted by `assign_role` above.)
    let idx2 = FilesystemPermissionIndex::open(dir.path()).expect("open #2");
    let allowed = idx2
        .check(owner, Action::Write, Resource::AppInstance(app))
        .await
        .expect("check");
    assert!(allowed, "owner binding must survive re-open");

    // Sanity-check `workspace_path` resolves to the same on-disk
    // file the first block wrote — the binding lives there.
    assert!(
        idx2.workspace_path(workspace).is_file(),
        "permissions.toml missing after re-open: {:?}",
        idx2.workspace_path(workspace)
    );
}
