//! Integration tests for `liquid-permissions`.
//!
//! Exercises the M3 plan-level success criterion (`IMPLEMENTATION_PLAN.md` §5.3):
//! an `AppViewer` agent cannot write, an `AppEditor` agent can, and a
//! `WorkspaceOwner` user can do both.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use liquid_core::{Action, AppInstanceId, LiquidError, PrincipalId, Resource, Result};
use liquid_permissions::{BuiltInRole, InMemoryPermissionIndex, PermissionIndex};

#[tokio::test]
async fn app_viewer_agent_cannot_write_to_app_instance() {
    let index = InMemoryPermissionIndex::new();
    let workspace = liquid_core::WorkspaceId::new();
    let agent = PrincipalId::new_agent();
    let app = AppInstanceId::new();

    index
        .assign_role(
            workspace,
            agent,
            BuiltInRole::AppViewer,
            Some(Resource::AppInstance(app)),
        )
        .await
        .expect("assign role");

    let read = index
        .check(agent, Action::Read, Resource::AppInstance(app))
        .await
        .expect("check read");
    assert!(read, "AppViewer must be able to read");

    let write = index
        .check(agent, Action::Write, Resource::AppInstance(app))
        .await
        .expect("check write");
    assert!(!write, "AppViewer must NOT be able to write");
}

#[tokio::test]
async fn app_editor_agent_can_write_to_app_instance() {
    let index = InMemoryPermissionIndex::new();
    let workspace = liquid_core::WorkspaceId::new();
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
        .check(agent, Action::Read, Resource::AppInstance(app))
        .await
        .expect("check read"));
    assert!(index
        .check(agent, Action::Write, Resource::AppInstance(app))
        .await
        .expect("check write"));
}

#[tokio::test]
async fn workspace_owner_can_read_and_write_anything_in_workspace() {
    let index = InMemoryPermissionIndex::new();
    let workspace = liquid_core::WorkspaceId::new();
    let owner = PrincipalId::new_user();
    let app = AppInstanceId::new();

    index
        .assign_role(workspace, owner, BuiltInRole::WorkspaceOwner, None)
        .await
        .expect("assign role");

    for action in [Action::Read, Action::Write, Action::Delete, Action::Admin] {
        assert!(
            index
                .check(owner, action, Resource::Workspace(workspace))
                .await
                .expect("check"),
            "owner must perform {action:?} on Workspace"
        );
        assert!(
            index
                .check(owner, action, Resource::AppInstance(app))
                .await
                .expect("check"),
            "owner must perform {action:?} on AppInstance"
        );
    }
}

#[tokio::test]
async fn app_viewer_scope_does_not_leak_to_other_app_instances() {
    let index = InMemoryPermissionIndex::new();
    let workspace = liquid_core::WorkspaceId::new();
    let agent = PrincipalId::new_agent();
    let app_a = AppInstanceId::new();
    let app_b = AppInstanceId::new();

    index
        .assign_role(
            workspace,
            agent,
            BuiltInRole::AppViewer,
            Some(Resource::AppInstance(app_a)),
        )
        .await
        .expect("assign role");

    assert!(index
        .check(agent, Action::Read, Resource::AppInstance(app_a))
        .await
        .expect("check"));
    assert!(!index
        .check(agent, Action::Read, Resource::AppInstance(app_b))
        .await
        .expect("check"));
}

#[tokio::test]
async fn revoke_role_removes_access() {
    let index = InMemoryPermissionIndex::new();
    let workspace = liquid_core::WorkspaceId::new();
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
    index
        .revoke_role(
            workspace,
            agent,
            BuiltInRole::AppEditor,
            Some(Resource::AppInstance(app)),
        )
        .await
        .expect("revoke role");

    assert!(!index
        .check(agent, Action::Write, Resource::AppInstance(app))
        .await
        .expect("check"));
}

#[tokio::test]
async fn agent_role_alone_grants_no_permissions() {
    let index = InMemoryPermissionIndex::new();
    let workspace = liquid_core::WorkspaceId::new();
    let agent = PrincipalId::new_agent();
    let app = AppInstanceId::new();

    index
        .assign_role(workspace, agent, BuiltInRole::Agent, None)
        .await
        .expect("assign role");

    assert!(!index
        .check(agent, Action::Read, Resource::AppInstance(app))
        .await
        .expect("check"));
    assert!(!index
        .check(agent, Action::Write, Resource::AppInstance(app))
        .await
        .expect("check"));
}

#[tokio::test]
async fn workspace_member_reads_anything_writes_pages_and_app_instances() {
    let index = InMemoryPermissionIndex::new();
    let workspace = liquid_core::WorkspaceId::new();
    let member = PrincipalId::new_user();
    let app = AppInstanceId::new();
    let page = liquid_core::PageId::new();

    index
        .assign_role(workspace, member, BuiltInRole::WorkspaceMember, None)
        .await
        .expect("assign role");

    assert!(index
        .check(member, Action::Read, Resource::Workspace(workspace))
        .await
        .expect("check"));
    assert!(index
        .check(member, Action::Write, Resource::Page(page))
        .await
        .expect("check"));
    assert!(index
        .check(member, Action::Write, Resource::AppInstance(app))
        .await
        .expect("check"));
    assert!(!index
        .check(member, Action::Admin, Resource::Workspace(workspace))
        .await
        .expect("check"));
}

#[tokio::test]
async fn assigning_app_scoped_role_without_scope_is_invalid_input() {
    let index = InMemoryPermissionIndex::new();
    let workspace = liquid_core::WorkspaceId::new();
    let agent = PrincipalId::new_agent();

    let err = index
        .assign_role(workspace, agent, BuiltInRole::AppViewer, None)
        .await
        .expect_err("scope must be required for AppViewer");

    assert!(matches!(err, LiquidError::InvalidInput(_)));
}

#[tokio::test]
async fn require_permission_macro_returns_forbidden_when_denied() {
    async fn caller(
        index: &InMemoryPermissionIndex,
        agent: PrincipalId,
        app: AppInstanceId,
    ) -> Result<()> {
        liquid_permissions::require_permission!(
            index,
            agent,
            Action::Write,
            Resource::AppInstance(app)
        );
        Ok(())
    }

    let index = InMemoryPermissionIndex::new();
    let workspace = liquid_core::WorkspaceId::new();
    let agent = PrincipalId::new_agent();
    let app = AppInstanceId::new();

    index
        .assign_role(
            workspace,
            agent,
            BuiltInRole::AppViewer,
            Some(Resource::AppInstance(app)),
        )
        .await
        .expect("assign role");

    let err = caller(&index, agent, app)
        .await
        .expect_err("write must be forbidden");
    assert!(matches!(err, LiquidError::Forbidden));
}

#[tokio::test]
async fn require_permission_macro_passes_when_allowed() {
    async fn caller(
        index: &InMemoryPermissionIndex,
        agent: PrincipalId,
        app: AppInstanceId,
    ) -> Result<()> {
        liquid_permissions::require_permission!(
            index,
            agent,
            Action::Write,
            Resource::AppInstance(app)
        );
        Ok(())
    }

    let index = InMemoryPermissionIndex::new();
    let workspace = liquid_core::WorkspaceId::new();
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

    caller(&index, agent, app).await.expect("write allowed");
}

#[tokio::test]
async fn workspace_owner_assignment_is_workspace_scoped() {
    // Owner of workspace A should not gain Owner privileges on workspace B.
    let index = InMemoryPermissionIndex::new();
    let ws_a = liquid_core::WorkspaceId::new();
    let ws_b = liquid_core::WorkspaceId::new();
    let user = PrincipalId::new_user();

    index
        .assign_role(ws_a, user, BuiltInRole::WorkspaceOwner, None)
        .await
        .expect("assign role");

    assert!(index
        .check(user, Action::Admin, Resource::Workspace(ws_a))
        .await
        .expect("check"));
    assert!(!index
        .check(user, Action::Admin, Resource::Workspace(ws_b))
        .await
        .expect("check"));
}

#[tokio::test]
async fn unassigned_principal_has_no_access() {
    let index = InMemoryPermissionIndex::new();
    let stranger = PrincipalId::new_agent();
    let app = AppInstanceId::new();

    assert!(!index
        .check(stranger, Action::Read, Resource::AppInstance(app))
        .await
        .expect("check"));
}

#[tokio::test]
async fn app_instance_bindings_in_two_workspaces_do_not_cross_match_distinct_uuids() {
    // The globally-unique-UUID assumption documented in
    // `IMPLEMENTATION_PLAN.md §4.2`: every `AppInstanceId` is
    // generated by `uuid::Uuid::new_v4()` and never reused across
    // workspaces. So a binding for `app_a` in workspace A cannot
    // match a check for `app_b` in workspace B, even though the
    // index's `workspace_matches` arm for non-Workspace resources is
    // workspace-agnostic — the UUIDs themselves disambiguate.
    let index = InMemoryPermissionIndex::new();
    let ws_a = liquid_core::WorkspaceId::new();
    let ws_b = liquid_core::WorkspaceId::new();
    let agent = PrincipalId::new_agent();
    let app_a = AppInstanceId::new();
    let app_b = AppInstanceId::new();
    assert_ne!(app_a, app_b, "uuid::new_v4 must yield distinct ids");

    index
        .assign_role(
            ws_a,
            agent,
            BuiltInRole::AppEditor,
            Some(Resource::AppInstance(app_a)),
        )
        .await
        .expect("assign workspace A binding");

    // The binding lets the agent edit `app_a`.
    assert!(index
        .check(agent, Action::Write, Resource::AppInstance(app_a))
        .await
        .expect("check"));
    // And does NOT let the agent edit `app_b` even though `app_b`
    // belongs to a different workspace (workspace B).
    assert!(!index
        .check(agent, Action::Write, Resource::AppInstance(app_b))
        .await
        .expect("check"));
    // Cross-workspace canary: there is no way to construct a
    // `Resource::AppInstance(app_a)` whose UUID belongs to workspace
    // B without the bridge layer mis-routing the call — `WorkspaceId`
    // is not part of `Resource::AppInstance`. The test below would
    // demonstrate a bug ONLY if the same UUID were used in two
    // workspaces, which `Uuid::new_v4` does not produce.
    let _ = ws_b;
}
