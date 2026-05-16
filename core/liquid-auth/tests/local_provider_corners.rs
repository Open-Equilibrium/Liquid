//! Coverage backfill for `liquid_auth::LocalIdentityProvider` — the
//! happy-path suite in `tests/local_provider.rs` and the wired-up
//! integration in `liquid-permissions/tests/m3_end_to_end.rs` together
//! cover the main flows, but a handful of the provider's "diagnostic
//! and configuration" surface (HMAC-length validation, on-disk path
//! getters, token-lifetime override) was previously unexercised by
//! any test. Catching a regression in those branches matters because
//! every higher-layer call into the bridge starts at the provider.
//!
//! Tests mirror the focused, single-assertion style of
//! `tests/local_provider.rs` — no shared fixtures.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use liquid_auth::{IdentityProvider, LocalIdentityProvider};
use liquid_core::LiquidError;
use tempfile::TempDir;

const SECRET: &[u8] = b"local-provider-corners-test-secret";

#[test]
fn new_rejects_short_hmac_secret() {
    // The provider requires a 16+ byte HMAC secret; a shorter secret
    // is a configuration mistake and must surface as `InvalidInput`,
    // not panic.
    //
    // `LocalIdentityProvider` does not impl `Debug`, so we cannot use
    // `expect_err`; pattern-match instead.
    let dir = TempDir::new().expect("tempdir");
    match LocalIdentityProvider::new(dir.path(), b"too-short") {
        Ok(_) => panic!("short HMAC secret must be rejected"),
        Err(LiquidError::InvalidInput(_)) => {}
        Err(other) => panic!("expected InvalidInput, got {other:?}"),
    }
}

#[test]
fn new_accepts_exactly_16_byte_secret() {
    // Boundary: exactly 16 bytes must succeed; the previous test
    // (`new_rejects_short_hmac_secret`) only proves shorter-than-16
    // fails. Without this case a future regression that bumps the
    // minimum to 17 would slip past.
    let dir = TempDir::new().expect("tempdir");
    let secret = b"sixteen-byte-key";
    assert_eq!(secret.len(), 16, "test fixture is no longer 16 bytes");
    LocalIdentityProvider::new(dir.path(), secret).expect("16-byte secret accepted");
}

#[test]
fn root_returns_constructor_argument() {
    let dir = TempDir::new().expect("tempdir");
    let p = LocalIdentityProvider::new(dir.path(), SECRET).expect("provider");
    assert_eq!(p.root(), dir.path());
}

#[test]
fn users_path_and_agents_path_resolve_under_root() {
    let dir = TempDir::new().expect("tempdir");
    let p = LocalIdentityProvider::new(dir.path(), SECRET).expect("provider");

    // Path getters must resolve relative to the root the provider
    // was opened with — bridge-layer callers feed these directly
    // into `cat` / `ls` for diagnostics.
    let users = p.users_path();
    let agents = p.agents_path();

    assert!(users.starts_with(dir.path()), "{users:?}");
    assert!(agents.starts_with(dir.path()), "{agents:?}");
    assert_eq!(
        users.file_name().and_then(|s| s.to_str()),
        Some("users.toml")
    );
    assert_eq!(
        agents.file_name().and_then(|s| s.to_str()),
        Some("agents.toml")
    );
}

#[tokio::test]
async fn with_token_lifetime_makes_tokens_expire_immediately() {
    // The lifetime override is used by tests and by short-lived
    // service-account tokens. Issue a zero-lifetime token, sleep one
    // second so wall-clock is unambiguous, and prove validate_token
    // refuses it.
    let dir = TempDir::new().expect("tempdir");
    let provider = LocalIdentityProvider::new(dir.path(), SECRET)
        .expect("provider")
        .with_token_lifetime(Duration::from_secs(0));

    let principal = provider
        .register_user("alice", "pw")
        .await
        .expect("register");
    let token = provider.issue_token(principal).await.expect("issue");

    // Same wall-clock second can race the expiry boundary; sleep past
    // it. `tokio::time::sleep` would be more idiomatic but the crate
    // does not enable tokio's `time` feature (see liquid-auth's
    // Cargo.toml — `features = ["macros", "rt"]`); pulling it in just
    // for one test is more churn than the blocking sleep is worth in
    // a single-test, current-thread runtime. The existing
    // `m3_walkthrough` uses the same pattern.
    std::thread::sleep(Duration::from_secs(1));
    let err = provider.validate_token(&token).await.expect_err("expired");
    // Every auth failure mode collapses to Forbidden (Absolute Rule:
    // never leak which mode failed — see §4.5).
    assert!(
        matches!(err, LiquidError::Forbidden),
        "expected Forbidden, got {err:?}"
    );
}
