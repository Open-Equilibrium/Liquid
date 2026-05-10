//! Integration tests for `FilesystemPermissionIndex`.
//!
//! TASK-007 (`IMPLEMENTATION_PLAN.md` §5.3, §9): a disk-backed
//! `PermissionIndex` whose role bindings persist as TOML at
//! `<root>/workspaces/<id>/permissions.toml`. Same trait, same matrix —
//! callers don't change.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use liquid_core::{Action, AppInstanceId, LiquidError, PrincipalId, Resource, WorkspaceId};
use liquid_permissions::{BuiltInRole, FilesystemPermissionIndex, PermissionIndex};
use tempfile::TempDir;

fn open(dir: &TempDir) -> FilesystemPermissionIndex {
    FilesystemPermissionIndex::open(dir.path()).expect("open index")
}

#[tokio::test]
async fn assign_then_check_round_trips() {
    let dir = TempDir::new().expect("tempdir");
    let index = open(&dir);
    let workspace = WorkspaceId::new();
    let agent = PrincipalId::new_agent();
    let app = AppInstanceId::new();

    index
        .assign_role(
            workspace,
            agent,
            BuiltInRole::AppEditor,
            Some(Resource::AppInstance(app)),
        )
        .await
        .expect("assign role");

    assert!(index
        .check(agent, Action::Write, Resource::AppInstance(app))
        .await
        .expect("check"));
}

#[tokio::test]
async fn assignment_persists_across_index_restart() {
    let dir = TempDir::new().expect("tempdir");
    let workspace = WorkspaceId::new();
    let agent = PrincipalId::new_agent();
    let app = AppInstanceId::new();

    {
        let index = open(&dir);
        index
            .assign_role(
                workspace,
                agent,
                BuiltInRole::AppEditor,
                Some(Resource::AppInstance(app)),
            )
            .await
            .expect("assign role");
    }

    // Re-open against the same root — bindings must reload from disk.
    let reopened = open(&dir);
    assert!(reopened
        .check(agent, Action::Write, Resource::AppInstance(app))
        .await
        .expect("check"));
}

#[tokio::test]
async fn revoke_persists_to_disk() {
    let dir = TempDir::new().expect("tempdir");
    let workspace = WorkspaceId::new();
    let agent = PrincipalId::new_agent();
    let app = AppInstanceId::new();

    {
        let index = open(&dir);
        index
            .assign_role(
                workspace,
                agent,
                BuiltInRole::AppEditor,
                Some(Resource::AppInstance(app)),
            )
            .await
            .expect("assign role");
        index
            .revoke_role(
                workspace,
                agent,
                BuiltInRole::AppEditor,
                Some(Resource::AppInstance(app)),
            )
            .await
            .expect("revoke role");
    }

    let reopened = open(&dir);
    assert!(!reopened
        .check(agent, Action::Write, Resource::AppInstance(app))
        .await
        .expect("check"));
}

#[tokio::test]
async fn scope_required_roles_rejected_when_scope_missing() {
    let dir = TempDir::new().expect("tempdir");
    let index = open(&dir);

    let err = index
        .assign_role(
            WorkspaceId::new(),
            PrincipalId::new_agent(),
            BuiltInRole::AppViewer,
            None,
        )
        .await
        .expect_err("AppViewer without scope must fail");

    assert!(matches!(err, LiquidError::InvalidInput(_)));
}

#[tokio::test]
async fn workspaces_persist_in_separate_files() {
    let dir = TempDir::new().expect("tempdir");
    let ws_a = WorkspaceId::new();
    let ws_b = WorkspaceId::new();
    let user = PrincipalId::new_user();

    {
        let index = open(&dir);
        index
            .assign_role(ws_a, user, BuiltInRole::WorkspaceOwner, None)
            .await
            .expect("assign A");
        index
            .assign_role(ws_b, user, BuiltInRole::WorkspaceMember, None)
            .await
            .expect("assign B");
    }

    let path_a = dir
        .path()
        .join("workspaces")
        .join(ws_a.to_string())
        .join("permissions.toml");
    let path_b = dir
        .path()
        .join("workspaces")
        .join(ws_b.to_string())
        .join("permissions.toml");
    assert!(path_a.exists(), "ws_a file must exist");
    assert!(path_b.exists(), "ws_b file must exist");

    let reopened = open(&dir);
    assert!(reopened
        .check(user, Action::Admin, Resource::Workspace(ws_a))
        .await
        .expect("admin A"));
    // WorkspaceMember does not grant Admin on its workspace.
    assert!(!reopened
        .check(user, Action::Admin, Resource::Workspace(ws_b))
        .await
        .expect("admin B"));
}

#[tokio::test]
async fn workspace_owner_scope_does_not_leak_across_workspaces_after_reload() {
    let dir = TempDir::new().expect("tempdir");
    let ws_a = WorkspaceId::new();
    let ws_b = WorkspaceId::new();
    let user = PrincipalId::new_user();

    {
        let index = open(&dir);
        index
            .assign_role(ws_a, user, BuiltInRole::WorkspaceOwner, None)
            .await
            .expect("assign owner of A");
    }

    let reopened = open(&dir);
    assert!(reopened
        .check(user, Action::Admin, Resource::Workspace(ws_a))
        .await
        .expect("admin A"));
    assert!(!reopened
        .check(user, Action::Admin, Resource::Workspace(ws_b))
        .await
        .expect("admin B (no binding)"));
}

#[tokio::test]
async fn opening_index_on_empty_root_succeeds() {
    let dir = TempDir::new().expect("tempdir");
    let index = open(&dir);

    // No bindings => everything denied.
    assert!(!index
        .check(
            PrincipalId::new_agent(),
            Action::Read,
            Resource::AppInstance(AppInstanceId::new()),
        )
        .await
        .expect("check"));
}

#[tokio::test]
async fn revoking_last_binding_leaves_well_formed_file() {
    let dir = TempDir::new().expect("tempdir");
    let ws = WorkspaceId::new();
    let user = PrincipalId::new_user();

    {
        let index = open(&dir);
        index
            .assign_role(ws, user, BuiltInRole::WorkspaceOwner, None)
            .await
            .expect("assign");
        index
            .revoke_role(ws, user, BuiltInRole::WorkspaceOwner, None)
            .await
            .expect("revoke");
    }

    // Reopening must succeed (parser must accept the empty-bindings file).
    let reopened = open(&dir);
    assert!(!reopened
        .check(user, Action::Admin, Resource::Workspace(ws))
        .await
        .expect("check"));
}

#[tokio::test]
async fn malformed_toml_returns_invalid_input_not_panic() {
    let dir = TempDir::new().expect("tempdir");
    let ws = WorkspaceId::new();
    let path = dir
        .path()
        .join("workspaces")
        .join(ws.to_string())
        .join("permissions.toml");
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&path, b"this is { definitely not valid toml [[").expect("write");

    let err =
        FilesystemPermissionIndex::open(dir.path()).expect_err("open must fail on malformed toml");
    assert!(matches!(err, LiquidError::InvalidInput(_)));
}
