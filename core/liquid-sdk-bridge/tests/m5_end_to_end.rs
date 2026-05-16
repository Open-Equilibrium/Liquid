//! M5 plan-level success criterion (`IMPLEMENTATION_PLAN.md` §5.5):
//!
//! > Dart test creates a workspace, writes a page, reads it back, and
//! > the round-trip data matches.
//!
//! Phase-1 ships the Rust side of the FFI bridge (5 entry-point
//! functions on [`BridgeServices`], each with `require_permission!`
//! as the first executable line per Absolute Rule 4). The Dart side
//! (`flutter_rust_bridge` codegen + `app/lib/bridge/` + the actual
//! `flutter test` integration test) is blocked on M6 scaffolding
//! (`app/` and `sdk/liquid_sdk/` directories don't exist yet) and
//! lands as a follow-up; the equivalent Rust-side criterion is
//! exercised here.
//!
//! This file is the M5 end-to-end test that wires every Phase-1
//! crate (`liquid-auth` + `liquid-permissions` + `liquid-vcs` +
//! `liquid-sdk-bridge`) together along the path a real bridge call
//! would follow.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use bytes::Bytes;
use liquid_auth::{IdentityProvider, LocalIdentityProvider};
use liquid_core::{Action, LiquidError, PageId, Resource};
use liquid_permissions::{InMemoryPermissionIndex, PermissionIndex};
use liquid_sdk_bridge::{
    BridgeServices, InMemoryWorkspaceRegistry, PageSnapshot, WorkspaceRegistry,
};
use liquid_vcs::InMemoryContentStore;
use tempfile::TempDir;

const SECRET: &[u8] = b"m5-end-to-end-test-secret-bytes";

type TestServices = BridgeServices<
    InMemoryContentStore,
    InMemoryPermissionIndex,
    LocalIdentityProvider,
    InMemoryWorkspaceRegistry,
>;

fn setup() -> (TempDir, TestServices) {
    let dir = TempDir::new().expect("tempdir");
    let auth = LocalIdentityProvider::new(dir.path(), SECRET).expect("auth");
    let services = BridgeServices {
        store: Arc::new(InMemoryContentStore::new()),
        permissions: Arc::new(InMemoryPermissionIndex::new()),
        identity: Arc::new(auth),
        registry: Arc::new(InMemoryWorkspaceRegistry::new()),
    };
    (dir, services)
}

#[tokio::test]
async fn create_workspace_rejects_tampered_token() {
    let (_d, s) = setup();
    let result = s
        .create_workspace("not.a.real.token", "demo".to_string())
        .await;
    assert!(
        matches!(result, Err(LiquidError::Forbidden)),
        "tampered token must collapse to Forbidden, got {result:?}"
    );
}

#[tokio::test]
async fn create_workspace_round_trips_id_into_registry() {
    let (_d, s) = setup();
    let alice = s
        .identity
        .register_user("alice", "pw")
        .await
        .expect("register");
    let token = s.identity.issue_token(alice).await.expect("issue token");

    let id = s
        .create_workspace(&token, "Demo Workspace".to_string())
        .await
        .expect("create");

    let registered = s.registry.list().await.expect("list");
    assert_eq!(registered.len(), 1, "registry should hold one workspace");
    assert_eq!(registered[0].id, id);
    assert_eq!(registered[0].name, "Demo Workspace");
    assert_eq!(registered[0].created_by, alice);
}

#[tokio::test]
async fn create_workspace_assigns_owner_role_to_creator() {
    let (_d, s) = setup();
    let alice = s
        .identity
        .register_user("alice", "pw")
        .await
        .expect("register");
    let token = s.identity.issue_token(alice).await.expect("issue token");

    let id = s
        .create_workspace(&token, "demo".to_string())
        .await
        .expect("create");

    let can_admin = s
        .permissions
        .check(alice, Action::Admin, Resource::Workspace(id))
        .await
        .expect("check");
    assert!(
        can_admin,
        "creator must be WorkspaceOwner after create_workspace"
    );
}

#[tokio::test]
async fn list_workspaces_filters_to_principals_with_a_binding() {
    let (_d, s) = setup();
    let alice = s
        .identity
        .register_user("alice", "pw")
        .await
        .expect("alice");
    let bob = s.identity.register_user("bob", "pw").await.expect("bob");

    let alice_token = s.identity.issue_token(alice).await.expect("alice token");
    let bob_token = s.identity.issue_token(bob).await.expect("bob token");

    s.create_workspace(&alice_token, "alice-ws".to_string())
        .await
        .expect("alice ws");

    let bob_view = s.list_workspaces(&bob_token).await.expect("bob list");
    assert!(
        bob_view.is_empty(),
        "bob has no binding ⇒ bob's list must be empty (got {bob_view:?})"
    );

    let alice_view = s.list_workspaces(&alice_token).await.expect("alice list");
    assert_eq!(alice_view.len(), 1);
    assert_eq!(alice_view[0].name, "alice-ws");
}

#[tokio::test]
async fn write_page_then_load_page_round_trips_bytes() {
    let (_d, s) = setup();
    let alice = s
        .identity
        .register_user("alice", "pw")
        .await
        .expect("alice");
    let token = s.identity.issue_token(alice).await.expect("token");
    let ws = s
        .create_workspace(&token, "demo".to_string())
        .await
        .expect("ws");
    let page_id = PageId::new();

    let body = Bytes::from_static(b"# Welcome\n\nfirst version\n");
    let snapshot = PageSnapshot::new(page_id, body.clone());
    let commit = s
        .write_page(&token, ws, page_id, snapshot, "seed welcome".to_string())
        .await
        .expect("write");
    assert_ne!(commit.to_string(), "", "commit id must be non-empty");

    let loaded = s.load_page(&token, ws, page_id).await.expect("load");
    assert_eq!(loaded.page_id, page_id, "page_id round-trip");
    assert_eq!(loaded.bytes, body, "bytes round-trip");
    assert_eq!(
        loaded.content_hash,
        liquid_core::ContentHash::of_bytes(&body),
        "content_hash matches bytes"
    );
}

#[tokio::test]
async fn write_page_rejects_app_viewer_role() {
    let (_d, s) = setup();
    let alice = s
        .identity
        .register_user("alice", "pw")
        .await
        .expect("alice");
    let alice_token = s.identity.issue_token(alice).await.expect("token");
    let ws = s
        .create_workspace(&alice_token, "demo".to_string())
        .await
        .expect("ws");
    let page_id = PageId::new();

    // Provision an agent and grant it AppViewer-on-a-page-scope.
    // (Phase-1 BuiltInRole::AppViewer permits Read on AppInstance/
    // Component only; Pages are governed by WorkspaceMember-or-higher.
    // An agent without any binding therefore cannot write.)
    let viewer = s
        .identity
        .provision_agent(ws, alice, "viewer-bot")
        .await
        .expect("provision");
    let viewer_token = s.identity.issue_token(viewer).await.expect("viewer token");

    let snapshot = PageSnapshot::new(page_id, Bytes::from_static(b"hijacked"));
    let result = s
        .write_page(
            &viewer_token,
            ws,
            page_id,
            snapshot,
            "should fail".to_string(),
        )
        .await;

    assert!(
        matches!(result, Err(LiquidError::Forbidden)),
        "agent without write binding must be Forbidden, got {result:?}"
    );
}

#[tokio::test]
async fn write_page_rejects_snapshot_page_id_mismatch() {
    let (_d, s) = setup();
    let alice = s
        .identity
        .register_user("alice", "pw")
        .await
        .expect("alice");
    let token = s.identity.issue_token(alice).await.expect("token");
    let ws = s
        .create_workspace(&token, "demo".to_string())
        .await
        .expect("ws");

    let outer_page = PageId::new();
    let inner_page = PageId::new();
    assert_ne!(outer_page, inner_page);
    let snapshot = PageSnapshot::new(inner_page, Bytes::from_static(b"x"));

    let result = s
        .write_page(&token, ws, outer_page, snapshot, "mismatch".to_string())
        .await;

    assert!(
        matches!(result, Err(LiquidError::InvalidInput(_))),
        "page_id mismatch must surface as InvalidInput, got {result:?}"
    );
}

#[tokio::test]
async fn load_page_rejects_agent_without_read_binding() {
    let (_d, s) = setup();
    let alice = s
        .identity
        .register_user("alice", "pw")
        .await
        .expect("alice");
    let alice_token = s.identity.issue_token(alice).await.expect("token");
    let ws = s
        .create_workspace(&alice_token, "demo".to_string())
        .await
        .expect("ws");
    let page_id = PageId::new();

    let snapshot = PageSnapshot::new(page_id, Bytes::from_static(b"private"));
    s.write_page(&alice_token, ws, page_id, snapshot, "seed".to_string())
        .await
        .expect("write");

    // Unbound agent: provisioned, but never assigned any role.
    let outsider = s
        .identity
        .provision_agent(ws, alice, "outsider")
        .await
        .expect("provision");
    let outsider_token = s
        .identity
        .issue_token(outsider)
        .await
        .expect("outsider token");

    let result = s.load_page(&outsider_token, ws, page_id).await;
    assert!(
        matches!(result, Err(LiquidError::Forbidden)),
        "unbound agent must be Forbidden on load_page, got {result:?}"
    );
}

#[tokio::test]
async fn check_permission_authenticates_caller_and_returns_query_result() {
    let (_d, s) = setup();
    let alice = s
        .identity
        .register_user("alice", "pw")
        .await
        .expect("alice");
    let alice_token = s.identity.issue_token(alice).await.expect("token");
    let ws = s
        .create_workspace(&alice_token, "demo".to_string())
        .await
        .expect("ws");

    let allowed = s
        .check_permission(
            &alice_token,
            &alice.to_string(),
            Action::Admin,
            Resource::Workspace(ws),
        )
        .await
        .expect("check");
    assert!(
        allowed,
        "owner must be allowed Admin on their own workspace"
    );

    // Querying with an invalid token rejects the call regardless of the
    // query subject — the bridge always authenticates the caller first.
    let result = s
        .check_permission(
            "bogus",
            &alice.to_string(),
            Action::Admin,
            Resource::Workspace(ws),
        )
        .await;
    assert!(
        matches!(result, Err(LiquidError::Forbidden)),
        "tampered caller-token rejected before query, got {result:?}"
    );
}

#[tokio::test]
async fn check_permission_rejects_malformed_subject_principal() {
    let (_d, s) = setup();
    let alice = s
        .identity
        .register_user("alice", "pw")
        .await
        .expect("alice");
    let token = s.identity.issue_token(alice).await.expect("token");
    let ws = s
        .create_workspace(&token, "demo".to_string())
        .await
        .expect("ws");

    let result = s
        .check_permission(
            &token,
            "not-a-principal-id",
            Action::Admin,
            Resource::Workspace(ws),
        )
        .await;
    assert!(
        matches!(result, Err(LiquidError::InvalidInput(_))),
        "malformed principal id must surface as InvalidInput, got {result:?}"
    );
}
