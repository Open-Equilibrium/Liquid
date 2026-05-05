//! Integration tests for `liquid-auth::LocalIdentityProvider`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use liquid_auth::{IdentityProvider, LocalIdentityProvider};
use liquid_core::{LiquidError, PrincipalId, WorkspaceId};
use tempfile::TempDir;

const SECRET: &[u8] = b"unit-test-hmac-secret-do-not-ship";

fn provider(dir: &TempDir) -> LocalIdentityProvider {
    LocalIdentityProvider::new(dir.path(), SECRET).expect("provider")
}

#[tokio::test]
async fn register_user_persists_to_users_toml() {
    let dir = TempDir::new().expect("tempdir");
    let p = provider(&dir);

    let principal = p
        .register_user("alice", "correct-horse-battery-staple")
        .await
        .expect("register");

    assert!(matches!(principal, PrincipalId::User(_)));
    assert!(dir.path().join("users.toml").exists());
}

#[tokio::test]
async fn register_user_rejects_duplicate_username() {
    let dir = TempDir::new().expect("tempdir");
    let p = provider(&dir);

    p.register_user("alice", "pw").await.expect("first");
    let err = p
        .register_user("alice", "pw2")
        .await
        .expect_err("second must fail");

    assert!(matches!(err, LiquidError::InvalidInput(_)));
}

#[tokio::test]
async fn register_user_rejects_empty_username_or_password() {
    let dir = TempDir::new().expect("tempdir");
    let p = provider(&dir);

    let err = p.register_user("", "pw").await.expect_err("empty user");
    assert!(matches!(err, LiquidError::InvalidInput(_)));

    let err = p.register_user("bob", "").await.expect_err("empty pw");
    assert!(matches!(err, LiquidError::InvalidInput(_)));
}

#[tokio::test]
async fn authenticate_user_returns_token_for_correct_password() {
    let dir = TempDir::new().expect("tempdir");
    let p = provider(&dir);
    let principal = p.register_user("alice", "pw").await.expect("register");

    let token = p
        .authenticate_user("alice", "pw")
        .await
        .expect("authenticate");

    let validated = p.validate_token(&token).await.expect("validate");
    assert_eq!(validated, principal);
}

#[tokio::test]
async fn authenticate_user_rejects_wrong_password() {
    let dir = TempDir::new().expect("tempdir");
    let p = provider(&dir);
    p.register_user("alice", "pw").await.expect("register");

    let err = p
        .authenticate_user("alice", "wrong")
        .await
        .expect_err("auth must fail");

    assert!(matches!(err, LiquidError::Forbidden));
}

#[tokio::test]
async fn authenticate_user_rejects_unknown_user() {
    let dir = TempDir::new().expect("tempdir");
    let p = provider(&dir);

    let err = p
        .authenticate_user("ghost", "pw")
        .await
        .expect_err("unknown user");

    assert!(matches!(err, LiquidError::Forbidden));
}

#[tokio::test]
async fn validate_token_rejects_tampered_token() {
    let dir = TempDir::new().expect("tempdir");
    let p = provider(&dir);
    p.register_user("alice", "pw").await.expect("register");
    let token = p.authenticate_user("alice", "pw").await.expect("auth");

    let mut bytes: Vec<char> = token.chars().collect();
    let last = bytes
        .iter()
        .rposition(char::is_ascii_hexdigit)
        .expect("hex char");
    bytes[last] = if bytes[last] == 'a' { 'b' } else { 'a' };
    let tampered: String = bytes.into_iter().collect();

    let err = p
        .validate_token(&tampered)
        .await
        .expect_err("tampered must fail");
    assert!(matches!(err, LiquidError::Forbidden));
}

#[tokio::test]
async fn validate_token_rejects_token_signed_with_wrong_secret() {
    let dir = TempDir::new().expect("tempdir");
    let p = provider(&dir);
    p.register_user("alice", "pw").await.expect("register");
    let token = p.authenticate_user("alice", "pw").await.expect("auth");

    let other =
        LocalIdentityProvider::new(dir.path(), b"different-secret").expect("other provider");
    let err = other
        .validate_token(&token)
        .await
        .expect_err("wrong secret must fail");
    assert!(matches!(err, LiquidError::Forbidden));
}

#[tokio::test]
async fn validate_token_rejects_expired_token() {
    let dir = TempDir::new().expect("tempdir");
    let p = LocalIdentityProvider::new(dir.path(), SECRET)
        .expect("provider")
        .with_token_lifetime(Duration::from_secs(0));
    let principal = p.register_user("alice", "pw").await.expect("register");
    let token = p.issue_token(principal).await.expect("issue");

    // Sleep ~1 second to ensure the token's `expires_unix` is in the past.
    std::thread::sleep(Duration::from_millis(1100));

    let err = p
        .validate_token(&token)
        .await
        .expect_err("expired must fail");
    assert!(matches!(err, LiquidError::Forbidden));
}

#[tokio::test]
async fn provision_agent_persists_to_agents_toml() {
    let dir = TempDir::new().expect("tempdir");
    let p = provider(&dir);
    let owner = p.register_user("alice", "pw").await.expect("register");
    let workspace = WorkspaceId::new();

    let agent = p
        .provision_agent(workspace, owner, "ci-bot")
        .await
        .expect("provision");

    assert!(matches!(agent, PrincipalId::Agent(_)));
    assert!(dir.path().join("agents.toml").exists());

    // The agent's id is recoverable across provider restarts.
    drop(p);
    let p2 = provider(&dir);
    let token = p2.issue_token(agent).await.expect("issue");
    assert_eq!(p2.validate_token(&token).await.expect("validate"), agent);
}

#[tokio::test]
async fn provision_agent_rejects_blank_name() {
    let dir = TempDir::new().expect("tempdir");
    let p = provider(&dir);
    let owner = p.register_user("alice", "pw").await.expect("register");
    let workspace = WorkspaceId::new();

    let err = p
        .provision_agent(workspace, owner, "")
        .await
        .expect_err("blank name");
    assert!(matches!(err, LiquidError::InvalidInput(_)));
}

#[tokio::test]
async fn users_toml_round_trips_across_provider_restart() {
    let dir = TempDir::new().expect("tempdir");
    let principal = {
        let p = provider(&dir);
        p.register_user("alice", "pw").await.expect("register")
    };

    let p2 = provider(&dir);
    let token = p2.authenticate_user("alice", "pw").await.expect("auth");
    let validated = p2.validate_token(&token).await.expect("validate");
    assert_eq!(validated, principal);
}

#[tokio::test]
async fn validate_token_rejects_malformed_token() {
    let dir = TempDir::new().expect("tempdir");
    let p = provider(&dir);
    p.register_user("alice", "pw").await.expect("register");

    for bad in ["", "not-a-token", "a.b.c", "a.b.c.d.e", "...."] {
        let err = p
            .validate_token(bad)
            .await
            .expect_err(&format!("'{bad}' must fail"));
        assert!(
            matches!(err, LiquidError::Forbidden | LiquidError::InvalidInput(_)),
            "got {err:?}"
        );
    }
}
