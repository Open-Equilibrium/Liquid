//! M3 plan-level success criterion (`IMPLEMENTATION_PLAN.md` §5.3):
//!
//! > Unit test proves an agent with `AppViewer` role cannot write; an agent
//! > with `AppEditor` role can; `WorkspaceOwner` can do both.
//!
//! This file wires `liquid-auth` and `liquid-permissions` together so the
//! same flow a real bridge call would follow — issue token → validate token
//! → `require_permission!` — is exercised end-to-end against persisted
//! credentials.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use liquid_auth::{IdentityProvider, LocalIdentityProvider};
use liquid_core::{Action, AppInstanceId, LiquidError, Resource, Result, WorkspaceId};
use liquid_permissions::{
    require_permission, BuiltInRole, InMemoryPermissionIndex, PermissionIndex,
};
use tempfile::TempDir;

const SECRET: &[u8] = b"m3-end-to-end-test-secret-bytes";

async fn try_write_app(
    permissions: &InMemoryPermissionIndex,
    auth: &LocalIdentityProvider,
    token: &str,
    app: AppInstanceId,
) -> Result<()> {
    let principal = auth.validate_token(token).await?;
    require_permission!(
        permissions,
        principal,
        Action::Write,
        Resource::AppInstance(app)
    );
    Ok(())
}

async fn try_read_app(
    permissions: &InMemoryPermissionIndex,
    auth: &LocalIdentityProvider,
    token: &str,
    app: AppInstanceId,
) -> Result<()> {
    let principal = auth.validate_token(token).await?;
    require_permission!(
        permissions,
        principal,
        Action::Read,
        Resource::AppInstance(app)
    );
    Ok(())
}

#[tokio::test]
async fn m3_app_viewer_cannot_write_app_editor_can_owner_can_both() {
    let dir = TempDir::new().expect("tempdir");
    let auth = LocalIdentityProvider::new(dir.path(), SECRET).expect("auth");
    let permissions = InMemoryPermissionIndex::new();

    // Bootstrap: a workspace, the human owner, and an app instance.
    let workspace = WorkspaceId::new();
    let owner = auth
        .register_user("alice", "owner-pw")
        .await
        .expect("owner");
    permissions
        .assign_role(workspace, owner, BuiltInRole::WorkspaceOwner, None)
        .await
        .expect("owner role");
    let app = AppInstanceId::new();

    // Two agents, provisioned by the owner; each gets a different role on
    // the same app instance.
    let viewer_agent = auth
        .provision_agent(workspace, owner, "viewer-bot")
        .await
        .expect("viewer agent");
    permissions
        .assign_role(
            workspace,
            viewer_agent,
            BuiltInRole::AppViewer,
            Some(Resource::AppInstance(app)),
        )
        .await
        .expect("viewer role");

    let editor_agent = auth
        .provision_agent(workspace, owner, "editor-bot")
        .await
        .expect("editor agent");
    permissions
        .assign_role(
            workspace,
            editor_agent,
            BuiltInRole::AppEditor,
            Some(Resource::AppInstance(app)),
        )
        .await
        .expect("editor role");

    // Tokens are issued via the IdentityProvider trait — same path the CLI
    // would use.
    let owner_token = auth.issue_token(owner).await.expect("owner token");
    let viewer_token = auth.issue_token(viewer_agent).await.expect("viewer token");
    let editor_token = auth.issue_token(editor_agent).await.expect("editor token");

    // (1) AppViewer agent CANNOT write.
    let err = try_write_app(&permissions, &auth, &viewer_token, app)
        .await
        .expect_err("AppViewer must not write");
    assert!(matches!(err, LiquidError::Forbidden));

    // (2) AppEditor agent CAN write.
    try_write_app(&permissions, &auth, &editor_token, app)
        .await
        .expect("AppEditor must write");

    // (3) WorkspaceOwner CAN both read and write.
    try_read_app(&permissions, &auth, &owner_token, app)
        .await
        .expect("Owner must read");
    try_write_app(&permissions, &auth, &owner_token, app)
        .await
        .expect("Owner must write");

    // Bonus: the AppViewer can still READ — proves the role is meaningful,
    // not just a "deny everything" placeholder.
    try_read_app(&permissions, &auth, &viewer_token, app)
        .await
        .expect("AppViewer must read");
}
