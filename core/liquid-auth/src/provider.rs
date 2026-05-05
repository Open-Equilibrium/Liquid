use async_trait::async_trait;
use liquid_core::{PrincipalId, Result, WorkspaceId};

/// Identity and session provider.
///
/// Trait shape mirrors `IMPLEMENTATION_PLAN.md` §4.5; errors normalise to
/// [`liquid_core::LiquidError`] for the same reason `ContentStore` and
/// `PermissionIndex` do (workspace-wide single-error-type policy in
/// `CLAUDE.md`).
#[async_trait]
pub trait IdentityProvider: Send + Sync {
    /// Validate a session token and return the principal it represents.
    /// Returns [`liquid_core::LiquidError::Forbidden`] on any failure
    /// (tampered, expired, malformed, unknown signing key) — never leak
    /// which mode of failure occurred.
    async fn validate_token(&self, token: &str) -> Result<PrincipalId>;

    /// Issue a short-lived signed session token for `principal`. The
    /// token's lifetime is configured per-provider.
    async fn issue_token(&self, principal: PrincipalId) -> Result<String>;

    /// Provision a new agent principal within `workspace`, recording
    /// `authorized_by` for audit. Returns the new agent's
    /// [`PrincipalId`]. The bridge layer is responsible for permission-
    /// gating this call before invoking it.
    async fn provision_agent(
        &self,
        workspace: WorkspaceId,
        authorized_by: PrincipalId,
        name: &str,
    ) -> Result<PrincipalId>;
}
