//! `--as <name|principal-id>` resolution.
//!
//! Translates the global `--as` flag into a short-lived bearer
//! token for the target principal, after verifying the calling
//! principal holds `Action::Admin` on the target's workspace
//! (per `IMPLEMENTATION_PLAN.md §5.8`: "every mutation runs
//! `require_permission!` first; impersonation still requires
//! matching token").
//!
//! Accepts two input shapes:
//!
//! - Principal-form (`a:<uuid>` / `agent:<uuid>` — `User`
//!   principals are not impersonable in Phase 1) — parsed via
//!   [`liquid_core::PrincipalId::FromStr`] and used directly.
//! - Bare agent name — must match exactly one provisioned agent
//!   across `$LIQUID_HOME/auth/agents.toml`. Zero matches →
//!   `NotFound`; multiple matches → `InvalidInput` (caller must
//!   disambiguate with the principal-form).
//!
//! On success returns the impersonation token; the caller writes
//! it back into `LIQUID_TOKEN` so downstream handlers see the
//! impersonated principal.

use std::path::Path;

use liquid_auth::IdentityProvider;
use liquid_core::{Action, LiquidError, PrincipalId, Resource, Result, WorkspaceId};
use liquid_permissions::PermissionIndex;

use crate::services::CliServices;
use crate::token;

/// Resolve `--as <target>` into a short-lived bearer token.
pub async fn resolve(services: &CliServices, home: &Path, target: &str) -> Result<String> {
    let caller_token = token::require(home)?;
    let caller = services.identity.validate_token(&caller_token).await?;

    let (target_principal, workspace) = match target.parse::<PrincipalId>() {
        Ok(p @ PrincipalId::Agent(_)) => {
            let summary = services.identity.find_agent_by_principal(p).await?;
            (summary.principal, summary.workspace)
        }
        Ok(PrincipalId::User(_)) => {
            return Err(LiquidError::InvalidInput(
                "--as for User principals is not allowed in Phase 1 (M7)".into(),
            ));
        }
        Err(_) => {
            // Bare name lookup — must match exactly one agent.
            let matches = services.identity.find_agents_by_name(target).await?;
            match matches.len() {
                0 => return Err(LiquidError::NotFound(format!("agent named '{target}'"))),
                1 => (matches[0].principal, matches[0].workspace),
                _ => {
                    return Err(LiquidError::InvalidInput(format!(
                        "multiple agents named '{target}'; use the agent's principal-form id (a:<uuid>)"
                    )));
                }
            }
        }
    };

    // Auth: caller must hold Admin on the target's workspace (or
    // be the target themselves). The §5.8 spec: impersonation
    // "still requires matching token" — i.e. some authority gate.
    if caller != target_principal {
        gate_caller_admin(services, caller, workspace).await?;
    }

    services.identity.issue_token(target_principal).await
}

async fn gate_caller_admin(
    services: &CliServices,
    caller: PrincipalId,
    workspace: WorkspaceId,
) -> Result<()> {
    let perms = services.permissions.as_ref();
    let allowed = perms
        .check(caller, Action::Admin, Resource::Workspace(workspace))
        .await?;
    if !allowed {
        return Err(LiquidError::Forbidden);
    }
    Ok(())
}
