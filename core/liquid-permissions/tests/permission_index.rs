//! Integration tests for `liquid-permissions`.
//!
//! Exercises the M3 plan-level success criterion (`IMPLEMENTATION_PLAN.md` §5.3):
//! an `AppViewer` agent cannot write, an `AppEditor` agent can, and a
//! `WorkspaceOwner` user can do both.
//!
//! Note on the cross-workspace UUID tests at the bottom of this file:
//! they *characterise* the §4.2 globally-unique-UUID tenant-isolation
//! assumption — they cannot enforce it from within `liquid-permissions`,
//! because `PermissionIndex::check` deliberately takes no
//! `WorkspaceId` for non-`Resource::Workspace` resources and because
//! `AppInstanceId::new` (which generates the actually-unique UUIDs)
//! lives in `liquid-core`. The tests therefore pin the *contract* —
//! a future refactor that adds workspace-strict checking for
//! `AppInstance` will break the second test, and §4.2 has to move
//! with it.

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
async fn distinct_app_instance_uuids_do_not_cross_match_per_binding() {
    // First half of the §4.2 contract: two distinct `AppInstanceId`s
    // (the common case — `Uuid::new_v4` makes collisions
    // astronomically unlikely) MUST be checked separately. A
    // binding for `app_a` never authorises an action on `app_b`,
    // regardless of which workspace originated either resource.
    let index = InMemoryPermissionIndex::new();
    let ws_a = liquid_core::WorkspaceId::new();
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

    assert!(index
        .check(agent, Action::Write, Resource::AppInstance(app_a))
        .await
        .expect("check"));
    assert!(!index
        .check(agent, Action::Write, Resource::AppInstance(app_b))
        .await
        .expect("check"));
}

#[tokio::test]
async fn app_instance_check_is_workspace_agnostic_by_uuid_uniqueness_assumption() {
    // Second half of the §4.2 contract — this is the assumption the
    // first test above relies on. `PermissionIndex::check` does NOT
    // take a `WorkspaceId` for non-`Resource::Workspace` resources;
    // the binding's stored workspace is informational for these
    // arms. So a binding made under workspace A for some
    // `AppInstance(x)` would authorise a check for the SAME UUID
    // initiated under workspace B context — IF a caller were ever
    // able to construct that situation.
    //
    // Tenant isolation therefore rests on the globally-unique-UUID
    // guarantee: every `AppInstanceId` is `Uuid::new_v4()` and never
    // reused across workspaces, so the cross-workspace match below
    // can only happen as a programming mistake at the bridge / CLI
    // boundary — never as a result of stored state. This test pins
    // the assumption: if a future refactor adds workspace-strict
    // checking for `AppInstance` it will break this test, and the
    // §4.2 contract documentation must move with it.
    let index = InMemoryPermissionIndex::new();
    let ws_a = liquid_core::WorkspaceId::new();
    let ws_b = liquid_core::WorkspaceId::new();
    let agent = PrincipalId::new_agent();
    let shared_app = AppInstanceId::new();

    index
        .assign_role(
            ws_a,
            agent,
            BuiltInRole::AppEditor,
            Some(Resource::AppInstance(shared_app)),
        )
        .await
        .expect("assign workspace A binding for shared_app");

    // The binding is keyed on `shared_app`'s UUID alone; the check
    // returns true regardless of which workspace context the bridge
    // *thought* it was acting on. This is the documented behaviour
    // — the workspace isolation guarantee comes from `Uuid::new_v4`,
    // not from the index walking workspace ids.
    let allowed = index
        .check(agent, Action::Write, Resource::AppInstance(shared_app))
        .await
        .expect("check");
    assert!(
        allowed,
        "non-Workspace resource checks are workspace-agnostic by design — \
         tenant isolation relies on UUID uniqueness, not on `check` walking \
         workspace ids (see IMPLEMENTATION_PLAN.md §4.2)."
    );
    let _ = ws_b;
}
