//! M5 manual-validation walkthrough — Phase-1 FFI bridge (Rust side)
//! (`IMPLEMENTATION_PLAN.md` §5.5).
//!
//! Self-asserting demonstration of [`BridgeServices`] composed from
//! every Phase-1 backend, exercised along the path a real Dart caller
//! would follow once `flutter_rust_bridge` codegen lands (TASK-012).
//!
//! Run it with:
//!
//! ```text
//! cargo run --manifest-path core/Cargo.toml -p liquid-sdk-bridge \
//!   --example m5_walkthrough
//! ```
//!
//! What the run proves:
//!
//! 1. `BridgeServices` is wired against the durable Phase-1 backends —
//!    `FilesystemContentStore` + `FilesystemPermissionIndex` +
//!    `LocalIdentityProvider` + the new
//!    `InMemoryWorkspaceRegistry` — under
//!    `${TMPDIR:-/tmp}/liquid-m5-walkthrough/`.
//! 2. `create_workspace` rejects a tampered token with
//!    `LiquidError::Forbidden`.
//! 3. `create_workspace` happy path: records the workspace in the
//!    registry AND assigns the caller `WorkspaceOwner` via the active
//!    `PermissionIndex` — verified by an `Action::Admin` check.
//! 4. `write_page` + `load_page` round-trip the bytes and the
//!    `PageSnapshot::content_hash` byte-for-byte.
//! 5. An agent provisioned by the owner but never assigned any
//!    role-binding is `Forbidden` on `load_page` AND `write_page`.
//! 6. `check_permission` rejects a tampered caller token even though
//!    the query subject is itself a legitimate user.
//! 7. `list_workspaces` for the owner returns the single workspace;
//!    for the outsider returns an empty list (per-row filtering by
//!    `PermissionIndex::check`).
//!
//! Every step uses `assert!()` / `assert_eq!()` so a panic === broken
//! milestone. Exit 0 ⇒ M5's Rust side still satisfies §5.5's
//! plan-level success criterion. On-disk artifacts under
//! `${TMPDIR:-/tmp}/liquid-m5-walkthrough/` are kept for inspection
//! (`ls -la` / `cat`); clean up with `just clean-walkthroughs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::Arc;

use bytes::Bytes;
use liquid_auth::{IdentityProvider, LocalIdentityProvider};
use liquid_core::{Action, LiquidError, PageId, Resource};
use liquid_permissions::FilesystemPermissionIndex;
use liquid_sdk_bridge::{
    BridgeServices, InMemoryWorkspaceRegistry, PageSnapshot, WorkspaceRegistry,
};
use liquid_vcs::FilesystemContentStore;

const SECRET: &[u8] = b"m5-walkthrough-secret-not-for-prod";

type RealServices = BridgeServices<
    FilesystemContentStore,
    FilesystemPermissionIndex,
    LocalIdentityProvider,
    InMemoryWorkspaceRegistry,
>;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    run_walkthrough().await;
}

async fn run_walkthrough() {
    let root = prepare_root();
    println!("M5 walkthrough — Rust-side FFI bridge against durable backends");
    println!("  root: {}", root.display());

    let services = build_services(&root);
    let owner_token = bootstrap_owner(&services).await;

    demo_tampered_token_is_forbidden(&services).await;
    let workspace = demo_create_workspace_assigns_owner_role(&services, &owner_token).await;
    let page_id = PageId::new();
    demo_write_then_load_round_trips_snapshot(&services, &owner_token, workspace, page_id).await;
    let outsider_token =
        demo_unbound_agent_is_forbidden(&services, &owner_token, workspace, page_id).await;
    demo_check_permission_rejects_tampered_caller(&services).await;
    demo_list_workspaces_filters_by_binding(&services, &owner_token, &outsider_token).await;

    println!();
    println!("M5 walkthrough OK");
    println!("Inspect the on-disk state:");
    println!("  ls -la {}", root.display());
}

/// Stable, cross-platform location so the reader can inspect the
/// on-disk state afterwards.
fn prepare_root() -> PathBuf {
    let root = std::env::temp_dir().join("liquid-m5-walkthrough");
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("clean prior run");
    }
    std::fs::create_dir_all(&root).expect("create root");
    root
}

fn build_services(root: &std::path::Path) -> RealServices {
    let auth = LocalIdentityProvider::new(root.join("auth"), SECRET).expect("auth");
    let store = FilesystemContentStore::open(root.join("vcs")).expect("store");
    let permissions = FilesystemPermissionIndex::open(root.join("perm")).expect("perm");
    let registry = InMemoryWorkspaceRegistry::new();
    BridgeServices {
        store: Arc::new(store),
        permissions: Arc::new(permissions),
        identity: Arc::new(auth),
        registry: Arc::new(registry),
    }
}

async fn bootstrap_owner(s: &RealServices) -> String {
    let alice = s
        .identity
        .register_user("alice", "owner-pw")
        .await
        .expect("register");
    println!("  register user alice -> {alice}");
    let token = s.identity.issue_token(alice).await.expect("issue");
    println!("  issued owner token (HMAC-SHA256, len={})", token.len());
    token
}

async fn demo_tampered_token_is_forbidden(s: &RealServices) {
    let bad = s
        .create_workspace("not.a.real.token", "tampered".to_string())
        .await;
    match bad {
        Err(LiquidError::Forbidden) => {
            println!("  create_workspace(tampered token) -> Forbidden (as expected)");
        }
        other => panic!("expected Forbidden, got {other:?}"),
    }
}

async fn demo_create_workspace_assigns_owner_role(
    s: &RealServices,
    owner_token: &str,
) -> liquid_core::WorkspaceId {
    let ws = s
        .create_workspace(owner_token, "Demo Workspace".to_string())
        .await
        .expect("create");
    println!("  create_workspace -> {ws} (name: Demo Workspace)");

    // Registry round-trip
    let listed = s.registry.list().await.expect("list");
    assert_eq!(
        listed.len(),
        1,
        "registry should hold exactly one workspace"
    );
    assert_eq!(listed[0].id, ws);

    // Owner-role check via the same PermissionIndex the bridge uses.
    let owner_principal = s
        .identity
        .validate_token(owner_token)
        .await
        .expect("validate");
    let can_admin = liquid_permissions::PermissionIndex::check(
        s.permissions.as_ref(),
        owner_principal,
        Action::Admin,
        Resource::Workspace(ws),
    )
    .await
    .expect("check");
    assert!(can_admin, "creator must hold WorkspaceOwner role");
    println!("  owner has Admin on {ws}: ✓");
    ws
}

async fn demo_write_then_load_round_trips_snapshot(
    s: &RealServices,
    owner_token: &str,
    workspace: liquid_core::WorkspaceId,
    page_id: PageId,
) {
    let body = Bytes::from_static(b"# Welcome\n\nM5 walkthrough page.\n");
    let snapshot = PageSnapshot::new(page_id, body.clone());

    let commit = s
        .write_page(
            owner_token,
            workspace,
            page_id,
            snapshot,
            "seed welcome".to_string(),
        )
        .await
        .expect("write");
    println!("  write_page {page_id} -> commit {commit}");

    let loaded = s
        .load_page(owner_token, workspace, page_id)
        .await
        .expect("load");
    assert_eq!(loaded.page_id, page_id);
    assert_eq!(loaded.bytes, body, "bytes round-trip");
    assert_eq!(
        loaded.content_hash,
        liquid_core::ContentHash::of_bytes(&body),
        "content_hash matches bytes"
    );
    println!(
        "  load_page  {page_id} -> {} bytes (content_hash matches)",
        loaded.bytes.len()
    );
}

async fn demo_unbound_agent_is_forbidden(
    s: &RealServices,
    owner_token: &str,
    workspace: liquid_core::WorkspaceId,
    page_id: PageId,
) -> String {
    let owner = s
        .identity
        .validate_token(owner_token)
        .await
        .expect("validate");
    let outsider = s
        .identity
        .provision_agent(workspace, owner, "outsider-bot")
        .await
        .expect("provision");
    let outsider_token = s.identity.issue_token(outsider).await.expect("issue");

    let load = s.load_page(&outsider_token, workspace, page_id).await;
    match load {
        Err(LiquidError::Forbidden) => {}
        other => panic!("expected Forbidden on load_page, got {other:?}"),
    }

    let snapshot = PageSnapshot::new(page_id, Bytes::from_static(b"hijacked"));
    let write = s
        .write_page(
            &outsider_token,
            workspace,
            page_id,
            snapshot,
            "should fail".to_string(),
        )
        .await;
    match write {
        Err(LiquidError::Forbidden) => {}
        other => panic!("expected Forbidden on write_page, got {other:?}"),
    }
    println!("  unbound agent -> Forbidden on both load_page and write_page");
    outsider_token
}

async fn demo_check_permission_rejects_tampered_caller(s: &RealServices) {
    let alice_principal = liquid_core::PrincipalId::new_user();
    let result = s
        .check_permission(
            "bogus.caller.token",
            &alice_principal.to_string(),
            Action::Admin,
            Resource::Workspace(liquid_core::WorkspaceId::new()),
        )
        .await;
    match result {
        Err(LiquidError::Forbidden) => {
            println!("  check_permission(bogus token) -> Forbidden (caller authenticated first)");
        }
        other => panic!("expected Forbidden, got {other:?}"),
    }
}

async fn demo_list_workspaces_filters_by_binding(
    s: &RealServices,
    owner_token: &str,
    outsider_token: &str,
) {
    let owner_view = s.list_workspaces(owner_token).await.expect("owner list");
    assert_eq!(owner_view.len(), 1, "owner should see the one workspace");
    let outsider_view = s
        .list_workspaces(outsider_token)
        .await
        .expect("outsider list");
    assert!(
        outsider_view.is_empty(),
        "outsider has no binding ⇒ empty list (got {outsider_view:?})"
    );
    println!(
        "  list_workspaces: owner sees {}, outsider sees {} (per-row filter by PermissionIndex::check)",
        owner_view.len(),
        outsider_view.len()
    );
}
