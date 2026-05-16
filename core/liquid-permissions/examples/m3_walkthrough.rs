//! M3 manual-validation walkthrough — Phase-1 auth + permissions
//! (`IMPLEMENTATION_PLAN.md` §5.3).
//!
//! Self-asserting demonstration of the four M3 surfaces wired together
//! along the exact path a real bridge call would take:
//!
//! ```text
//! issue_token → validate_token → require_permission! → ✓ or Forbidden
//! ```
//!
//! Run it with:
//!
//! ```text
//! cargo run --manifest-path core/Cargo.toml -p liquid-permissions \
//!   --example m3_walkthrough
//! ```
//!
//! What the run proves:
//!
//! 1. `LocalIdentityProvider` registers a user (`alice`) under a
//!    durable `users.toml` (Argon2id hashes).
//! 2. The same provider provisions two agent principals under a
//!    durable `agents.toml`.
//! 3. `issue_token` produces a `principal . expires_unix . hmac_hex`
//!    triple; `validate_token` round-trips back to the same
//!    `PrincipalId` (matches §4.5).
//! 4. The plan-level success criterion holds end-to-end:
//!    - `AppViewer` agent CANNOT write an `AppInstance` (`Forbidden`).
//!    - `AppEditor` agent CAN write.
//!    - `WorkspaceOwner` CAN do both.
//! 5. The same demonstration repeated against
//!    `FilesystemPermissionIndex` proves disk-persisted role bindings
//!    survive a fresh process (re-opens the same root and re-runs the
//!    checks).
//! 6. Tampered, malformed, expired, and wrong-signing-key tokens
//!    all collapse to `LiquidError::Forbidden` (no mode-leak —
//!    Absolute Rule from §4.5).
//!
//! On-disk artifacts are kept under `/tmp/liquid-m3-walkthrough/` for
//! human inspection after the run. Exit 0 ⇒ M3 still satisfies its
//! plan-level success criterion.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use liquid_auth::{IdentityProvider, LocalIdentityProvider};
use liquid_core::{Action, AppInstanceId, LiquidError, Resource, Result, WorkspaceId};
use liquid_permissions::{
    require_permission, BuiltInRole, FilesystemPermissionIndex, InMemoryPermissionIndex,
    PermissionIndex,
};

const SECRET: &[u8] = b"m3-walkthrough-secret-not-for-prod";

/// Helper that mirrors the bridge-layer call shape:
/// `validate_token` → `require_permission!(...)`.
async fn try_write_app(
    permissions: &impl PermissionIndex,
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
    permissions: &impl PermissionIndex,
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

#[tokio::main(flavor = "current_thread")]
async fn main() {
    run_walkthrough().await;
}

/// Bundle of principals + tokens the walkthrough threads through every
/// subsequent phase. Keeping it in one struct lets each phase be a
/// small, named function with a tidy argument list.
struct Actors {
    workspace: WorkspaceId,
    app: AppInstanceId,
    owner: liquid_core::PrincipalId,
    viewer_agent: liquid_core::PrincipalId,
    editor_agent: liquid_core::PrincipalId,
    owner_token: String,
    viewer_token: String,
    editor_token: String,
}

/// Top-level orchestrator. Split out of `main` so each phase of the
/// walkthrough is a single, named function — easier to read, easier
/// to keep under clippy's `too_many_lines` ceiling without an
/// `#[allow]` escape hatch.
async fn run_walkthrough() {
    let (auth_root, perm_root) = prepare_roots();
    let auth = LocalIdentityProvider::new(&auth_root, SECRET).expect("auth");
    let actors = bootstrap_actors(&auth).await;

    exercise_in_memory_index(&auth, &actors).await;
    let perm_file = exercise_filesystem_index(&auth, &perm_root, &actors).await;
    exercise_token_negative_surface(&auth, &auth_root, &actors).await;
    print_inspection_hints(&auth, &perm_file);
}

/// Stable, cross-platform location so the reader can inspect the
/// on-disk state afterwards. Returns `(auth_root, perm_root)`.
fn prepare_roots() -> (std::path::PathBuf, std::path::PathBuf) {
    let root = std::env::temp_dir().join("liquid-m3-walkthrough");
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("clean prior run");
    }
    std::fs::create_dir_all(&root).expect("create root");
    let auth_root = root.join("auth");
    let perm_root = root.join("perm");
    std::fs::create_dir_all(&auth_root).expect("auth root");
    std::fs::create_dir_all(&perm_root).expect("perm root");
    println!("M3 walkthrough — auth + permissions");
    println!("  root: {}", root.display());
    (auth_root, perm_root)
}

/// Register the workspace owner, provision the two agent principals,
/// issue + round-trip-verify all three tokens.
async fn bootstrap_actors(auth: &LocalIdentityProvider) -> Actors {
    let workspace = WorkspaceId::new();
    let app = AppInstanceId::new();
    println!("  workspace: {workspace}");
    println!("  app:       {app}");

    let owner = auth
        .register_user("alice", "owner-pw")
        .await
        .expect("owner");
    println!("  register user alice -> {owner}");

    let viewer_agent = auth
        .provision_agent(workspace, owner, "viewer-bot")
        .await
        .expect("viewer agent");
    let editor_agent = auth
        .provision_agent(workspace, owner, "editor-bot")
        .await
        .expect("editor agent");
    println!("  provision viewer-bot -> {viewer_agent}");
    println!("  provision editor-bot -> {editor_agent}");

    let owner_token = auth.issue_token(owner).await.expect("owner token");
    let viewer_token = auth.issue_token(viewer_agent).await.expect("viewer token");
    let editor_token = auth.issue_token(editor_agent).await.expect("editor token");

    let principal_back = auth.validate_token(&viewer_token).await.expect("validate");
    assert_eq!(
        principal_back, viewer_agent,
        "round-trip principal mismatch"
    );
    println!("  token format: <principal>.<expires_unix>.<hmac_hex> — round-trip ok");

    Actors {
        workspace,
        app,
        owner,
        viewer_agent,
        editor_agent,
        owner_token,
        viewer_token,
        editor_token,
    }
}

/// Plan-level criterion: viewer cannot write, editor can, owner can both.
async fn exercise_in_memory_index(auth: &LocalIdentityProvider, a: &Actors) {
    println!("  --- InMemoryPermissionIndex ---");
    let im = InMemoryPermissionIndex::new();
    seed_role_bindings(&im, a).await;

    assert_forbidden(
        try_write_app(&im, auth, &a.viewer_token, a.app).await,
        "viewer write",
    );
    try_write_app(&im, auth, &a.editor_token, a.app)
        .await
        .expect("editor write");
    try_read_app(&im, auth, &a.owner_token, a.app)
        .await
        .expect("owner read");
    try_write_app(&im, auth, &a.owner_token, a.app)
        .await
        .expect("owner write");
    try_read_app(&im, auth, &a.viewer_token, a.app)
        .await
        .expect("viewer read");
    println!("  in-memory matrix: viewer write=Forbidden  editor write=OK  owner read+write=OK  viewer read=OK");
}

/// Same criterion but with bindings persisted to disk; the second
/// `open` simulates a fresh process. Returns the path of the
/// on-disk `permissions.toml` so the inspection-hints phase can
/// print it.
async fn exercise_filesystem_index(
    auth: &LocalIdentityProvider,
    perm_root: &std::path::Path,
    a: &Actors,
) -> std::path::PathBuf {
    println!("  --- FilesystemPermissionIndex (durable) ---");
    {
        let fs_idx = FilesystemPermissionIndex::open(perm_root).expect("open fs idx");
        seed_role_bindings(&fs_idx, a).await;
        // Drop the index so the on-disk TOML is fully closed.
    }
    let perm_file = perm_root
        .join("workspaces")
        .join(a.workspace.to_string())
        .join("permissions.toml");
    assert!(
        perm_file.is_file(),
        "missing on-disk permissions: {}",
        perm_file.display()
    );

    let fs_idx2 = FilesystemPermissionIndex::open(perm_root).expect("reopen fs idx");
    assert_forbidden(
        try_write_app(&fs_idx2, auth, &a.viewer_token, a.app).await,
        "viewer write (fs)",
    );
    try_write_app(&fs_idx2, auth, &a.editor_token, a.app)
        .await
        .expect("editor write (fs)");
    try_write_app(&fs_idx2, auth, &a.owner_token, a.app)
        .await
        .expect("owner write (fs)");
    println!("  fs matrix after reopen: viewer write=Forbidden  editor write=OK  owner write=OK");
    perm_file
}

/// Shared owner / viewer / editor role-binding seed used by both the
/// in-memory and filesystem index phases.
async fn seed_role_bindings(idx: &impl PermissionIndex, a: &Actors) {
    idx.assign_role(a.workspace, a.owner, BuiltInRole::WorkspaceOwner, None)
        .await
        .expect("owner role");
    idx.assign_role(
        a.workspace,
        a.viewer_agent,
        BuiltInRole::AppViewer,
        Some(Resource::AppInstance(a.app)),
    )
    .await
    .expect("viewer role");
    idx.assign_role(
        a.workspace,
        a.editor_agent,
        BuiltInRole::AppEditor,
        Some(Resource::AppInstance(a.app)),
    )
    .await
    .expect("editor role");
}

/// Tampered, malformed, wrong-key, and expired tokens all collapse
/// to `Forbidden` — no mode leak (§4.5).
async fn exercise_token_negative_surface(
    auth: &LocalIdentityProvider,
    auth_root: &std::path::Path,
    a: &Actors,
) {
    let tampered = format!("{}xx", a.owner_token);
    assert_forbidden(
        auth.validate_token(&tampered).await.map(|_| ()),
        "tampered token",
    );
    let wrong_key_provider =
        LocalIdentityProvider::new(auth_root, b"different-secret").expect("alt provider");
    assert_forbidden(
        wrong_key_provider
            .validate_token(&a.owner_token)
            .await
            .map(|_| ()),
        "wrong signing key",
    );
    let malformed = "not.a.token";
    assert_forbidden(
        auth.validate_token(malformed).await.map(|_| ()),
        "malformed token",
    );
    // Expired: zero-lifetime provider issues a token already past
    // `expires_unix`; same secret so HMAC matches, only the expiry
    // field fails.
    let short_lived = LocalIdentityProvider::new(auth_root, SECRET)
        .expect("short-lived provider")
        .with_token_lifetime(Duration::from_secs(0));
    let expired_token = short_lived
        .issue_token(a.owner)
        .await
        .expect("issue expired");
    // Real-world clocks may issue the token in the same second it is
    // validated; sleep a beat so `expires_unix < now` for sure.
    std::thread::sleep(Duration::from_secs(1));
    assert_forbidden(
        auth.validate_token(&expired_token).await.map(|_| ()),
        "expired token",
    );
    println!(
        "  token negatives: tampered=Forbidden  wrong-key=Forbidden  malformed=Forbidden  expired=Forbidden"
    );
}

fn print_inspection_hints(auth: &LocalIdentityProvider, perm_file: &std::path::Path) {
    let users_path = auth.users_path();
    let agents_path = auth.agents_path();
    println!();
    println!("M3 walkthrough OK");
    println!("Inspect the on-disk state:");
    println!("  cat {}", users_path.display());
    println!("  cat {}", agents_path.display());
    println!("  cat {}", perm_file.display());
}

/// Assert that a `Result` is `Err(LiquidError::Forbidden)`. Anything else
/// — including a different `LiquidError` variant — is a milestone
/// regression and panics with a context message.
fn assert_forbidden(result: Result<()>, label: &str) {
    match result {
        Err(LiquidError::Forbidden) => {}
        Err(other) => panic!("{label}: expected Forbidden, got {other:?}"),
        Ok(()) => panic!("{label}: expected Forbidden, got Ok(())"),
    }
}
