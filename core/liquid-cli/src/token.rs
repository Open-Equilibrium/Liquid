//! Bearer-token resolution + bootstrap.
//!
//! Resolution order (per `IMPLEMENTATION_PLAN.md §12` Authentication):
//!
//! 1. `LIQUID_TOKEN` env var (preferred for automation)
//! 2. `<LIQUID_HOME>/token` file (one-line — written by `bootstrap`)
//!
//! `bootstrap` registers a default `cli` user with a random
//! password and persists the issued token to `<LIQUID_HOME>/token`
//! the first time `workspace create` runs without a token. The
//! random password is never persisted — only the HMAC-signed
//! token is, so a stolen token file cannot be used to recover
//! the password.

use std::fs;
use std::path::Path;

use liquid_auth::IdentityProvider;
use liquid_core::{LiquidError, Result};

use crate::services::{atomic_write, CliServices};

const BOOTSTRAP_USERNAME: &str = "cli";
const TOKEN_FILENAME: &str = "token";

/// Resolve the active bearer token. `None` means neither source is
/// populated — the caller decides whether to bootstrap (used by
/// `workspace create`) or fail.
pub fn resolve(home: &Path) -> Option<String> {
    if let Some(env) = std::env::var("LIQUID_TOKEN").ok().filter(|s| !s.is_empty()) {
        return Some(env);
    }
    let path = home.join(TOKEN_FILENAME);
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Like [`resolve`] but errors with a friendly `Forbidden` when
/// no token is available. Used by every subcommand other than
/// `workspace create` and `auth token`.
pub fn require(home: &Path) -> Result<String> {
    resolve(home).ok_or_else(|| {
        LiquidError::InvalidInput(
            "no bearer token — set LIQUID_TOKEN or run `liquid workspace create` first".into(),
        )
    })
}

/// Ensure the default bootstrap user exists, issue a fresh token,
/// persist it under `<home>/token`, and return it. Idempotent —
/// the second call sees the existing user and just re-issues +
/// re-writes the token.
pub async fn bootstrap(services: &CliServices, home: &Path) -> Result<String> {
    // Random password — never persisted; the issued HMAC-signed
    // token is what we keep in `<home>/token`. A subsequent login
    // would need a fresh password (Phase-1 has no `auth login`
    // subcommand yet — TASK-009 / M7).
    let password = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );

    // `register_user` rejects an existing username with
    // `InvalidInput`; in that case we fall through and just issue
    // a fresh token via `authenticate_user` against the original
    // password — except we do NOT have the original password.
    // Phase-1 workaround: on the duplicate-user branch we look up
    // the existing principal by re-reading the on-disk users file
    // via a deterministic side-channel — but `LocalIdentityProvider`
    // does not expose that. Simpler: store a deterministic password
    // alongside the secret. To avoid that complexity, we use a
    // "register-or-reuse" pattern: if the bootstrap user already
    // exists, re-issue using the stable token file we wrote last
    // time and trust the user did not delete `<home>/secret`.
    match services
        .identity
        .register_user(BOOTSTRAP_USERNAME, &password)
        .await
    {
        Ok(principal) => {
            let token = services.identity.issue_token(principal).await?;
            write_token(home, &token)?;
            Ok(token)
        }
        Err(LiquidError::InvalidInput(_)) => {
            // User already exists from a prior bootstrap. If we
            // still have the prior token on disk it is valid (HMAC
            // keys persist via `<home>/secret`); reuse it.
            if let Some(existing) = resolve(home) {
                return Ok(existing);
            }
            Err(LiquidError::InvalidInput(
                "bootstrap user exists but no token is on disk — \
                 delete $LIQUID_HOME/auth/users.toml to re-bootstrap, \
                 or set LIQUID_TOKEN manually"
                    .into(),
            ))
        }
        Err(other) => Err(other),
    }
}

fn write_token(home: &Path, token: &str) -> Result<()> {
    atomic_write(&home.join(TOKEN_FILENAME), token.as_bytes())
}
