use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use argon2::password_hash::{rand_core::OsRng, PasswordHash, SaltString};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use async_trait::async_trait;
use liquid_core::{LiquidError, PrincipalId, Result, WorkspaceId};
use uuid::Uuid;

use crate::provider::IdentityProvider;
use crate::storage::{
    agents_path, ensure_root, load_agents, load_users, principal_to_string, save_agents,
    save_users, users_path, workspace_uuid, AgentRecord, UserRecord,
};
use crate::token::{build_token, now_unix, parse_and_verify_token, TokenPayload};

const DEFAULT_TOKEN_LIFETIME: Duration = Duration::from_secs(60 * 60); // 1 hour

/// File-backed local identity provider.
///
/// Stores users at `<root>/users.toml` (Argon2id-hashed passwords) and
/// provisioned agents at `<root>/agents.toml`. Sessions are HMAC-SHA256
/// tokens of the form `principal . expires_unix . hmac_hex`.
///
/// Phase 1 only — Phase 3 swaps in OIDC (`IMPLEMENTATION_PLAN.md` §9).
/// Application callers depend on the [`IdentityProvider`] trait so the swap
/// is transparent.
pub struct LocalIdentityProvider {
    root: PathBuf,
    secret: Vec<u8>,
    token_lifetime: Duration,
    state: Mutex<()>,
}

impl LocalIdentityProvider {
    /// Open (or initialise) a local provider rooted at `root`. `secret` is
    /// the HMAC signing key — at least 16 bytes; in production this comes
    /// from `~/.liquid/auth/hmac_secret`.
    pub fn new(root: impl Into<PathBuf>, secret: &[u8]) -> Result<Self> {
        let root = root.into();
        if secret.len() < 16 {
            return Err(LiquidError::InvalidInput(
                "HMAC secret must be at least 16 bytes".into(),
            ));
        }
        ensure_root(&root)?;
        Ok(Self {
            root,
            secret: secret.to_vec(),
            token_lifetime: DEFAULT_TOKEN_LIFETIME,
            state: Mutex::new(()),
        })
    }

    /// Override the default 1-hour token lifetime. Mainly for tests.
    #[must_use]
    pub fn with_token_lifetime(mut self, lifetime: Duration) -> Self {
        self.token_lifetime = lifetime;
        self
    }

    /// Register a new local user, hashing the password with Argon2id.
    /// Returns the new user's [`PrincipalId`].
    ///
    /// Async even though the local backend doesn't await — the trait-level
    /// API will become async in Phase 3 (OIDC / remote user store) and
    /// callers shouldn't have to choose between sync/async paths.
    #[allow(clippy::unused_async)]
    pub async fn register_user(&self, username: &str, password: &str) -> Result<PrincipalId> {
        if username.trim().is_empty() {
            return Err(LiquidError::InvalidInput("username is empty".into()));
        }
        if password.is_empty() {
            return Err(LiquidError::InvalidInput("password is empty".into()));
        }

        let guard = self.state.lock().map_err(poisoned)?;
        let mut users = load_users(&self.root)?;
        if users.iter().any(|u| u.username == username) {
            return Err(LiquidError::InvalidInput(format!(
                "username already registered: {username}"
            )));
        }
        let hash = hash_password(password)?;
        let id = Uuid::new_v4();
        users.push(UserRecord {
            id,
            username: username.to_owned(),
            password_hash: hash,
        });
        save_users(&self.root, &users)?;
        drop(guard);
        Ok(PrincipalId::User(id))
    }

    /// Verify a username + password against the stored Argon2id hash and
    /// issue a fresh session token on success.
    pub async fn authenticate_user(&self, username: &str, password: &str) -> Result<String> {
        let principal = {
            let guard = self.state.lock().map_err(poisoned)?;
            let users = load_users(&self.root)?;
            let record = users
                .iter()
                .find(|u| u.username == username)
                .ok_or(LiquidError::Forbidden)?;
            verify_password(password, &record.password_hash)?;
            let id = record.id;
            drop(guard);
            PrincipalId::User(id)
        };
        self.issue_token(principal).await
    }

    /// Path to the on-disk users file (for tests / diagnostics).
    pub fn users_path(&self) -> PathBuf {
        users_path(&self.root)
    }

    /// Path to the on-disk agents file (for tests / diagnostics).
    pub fn agents_path(&self) -> PathBuf {
        agents_path(&self.root)
    }

    /// Root directory the provider was constructed with.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[async_trait]
impl IdentityProvider for LocalIdentityProvider {
    async fn validate_token(&self, token: &str) -> Result<PrincipalId> {
        parse_and_verify_token(token, &self.secret)
    }

    async fn issue_token(&self, principal: PrincipalId) -> Result<String> {
        let expires_unix = now_unix().saturating_add(self.token_lifetime.as_secs());
        build_token(
            &TokenPayload {
                principal,
                expires_unix,
            },
            &self.secret,
        )
    }

    async fn provision_agent(
        &self,
        workspace: WorkspaceId,
        authorized_by: PrincipalId,
        name: &str,
    ) -> Result<PrincipalId> {
        if name.trim().is_empty() {
            return Err(LiquidError::InvalidInput("agent name is empty".into()));
        }
        let guard = self.state.lock().map_err(poisoned)?;
        let mut agents = load_agents(&self.root)?;
        let id = Uuid::new_v4();
        agents.push(AgentRecord {
            id,
            name: name.to_owned(),
            workspace_id: workspace_uuid(workspace),
            authorized_by: principal_to_string(authorized_by),
            created_unix: now_unix(),
        });
        save_agents(&self.root, &agents)?;
        drop(guard);
        Ok(PrincipalId::Agent(id))
    }
}

fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| LiquidError::InvalidInput(format!("argon2 hash error: {e}")))
}

fn verify_password(password: &str, stored_hash: &str) -> Result<()> {
    let parsed = PasswordHash::new(stored_hash).map_err(|_| LiquidError::Forbidden)?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| LiquidError::Forbidden)
}

fn poisoned<T>(_: PoisonError<T>) -> LiquidError {
    LiquidError::InvalidInput("auth state lock poisoned".into())
}
