//! `liquid auth …` subcommands.

use std::path::Path;

use liquid_auth::IdentityProvider;
use liquid_core::{LiquidError, Resource, Result};
use liquid_permissions::{BuiltInRole, PermissionIndex};
use serde_json::json;
use uuid::Uuid;

use crate::cmd::parse;
use crate::output::Envelope;
use crate::services::CliServices;
use crate::token;

/// `liquid auth provision-agent <name> --workspace <id> --role <role>`
/// — provisions a fresh agent principal under the caller's
/// workspace and issues a session token for it. The caller must
/// hold `Action::Admin` on `Resource::Workspace(workspace)`
/// (`WorkspaceOwner` does; `WorkspaceMember` does not — same
/// matrix as `liquid_permissions::BuiltInRole::permits`).
pub async fn provision_agent(
    services: &CliServices,
    home: &Path,
    name: String,
    workspace: &str,
    role: &str,
    scope: Option<&str>,
) -> Result<Envelope> {
    let caller_token = token::require(home)?;
    let caller = services.identity.validate_token(&caller_token).await?;

    let workspace = parse::workspace_id(workspace)?;
    let role = parse_role(role)?;
    let scope = parse_scope(scope, role)?;

    // Authorise: caller must hold Admin on the workspace.
    let perms = services.permissions.as_ref();
    let allowed = perms
        .check(
            caller,
            liquid_core::Action::Admin,
            Resource::Workspace(workspace),
        )
        .await?;
    if !allowed {
        return Err(LiquidError::Forbidden);
    }

    let agent = services
        .identity
        .provision_agent(workspace, caller, &name)
        .await?;
    perms.assign_role(workspace, agent, role, scope).await?;
    let token = services.identity.issue_token(agent).await?;

    // Emit the bare UUID under `agent_id` to mirror
    // `data.workspace_id`; emit the full principal-form string
    // (`agent:<uuid>`) under `principal` so callers wanting the
    // wire form do not have to re-assemble it.
    let agent_uuid = match agent {
        liquid_core::PrincipalId::Agent(u) => u,
        liquid_core::PrincipalId::User(_) => {
            return Err(LiquidError::InvalidInput(
                "provision_agent unexpectedly returned a User principal".into(),
            ));
        }
    };
    let summary = format!("provisioned agent {agent} with role {role:?}");
    Ok(Envelope::ok_data(json!({
        "agent_id": agent_uuid.to_string(),
        "principal": agent.to_string(),
        "role": format!("{role:?}"),
        "token": token,
    }))
    .with_text(summary))
}

/// `liquid auth token` — print the active bearer token. Useful for
/// piping into `$LIQUID_TOKEN` in shell scripts.
pub fn token(home: &Path) -> Result<Envelope> {
    let t = token::require(home)?;
    Ok(Envelope::ok_data(json!({ "token": t.clone() })).with_text(t))
}

/// `liquid auth login --username <u> --password <p> [--register]`
/// — non-interactive auth. With `--register` first creates the
/// user (rejects if the username is taken); without it,
/// authenticates an existing user. On success writes the token to
/// `$LIQUID_HOME/token` so subsequent commands pick it up.
pub async fn login(
    services: &CliServices,
    home: &Path,
    username: &str,
    password: &str,
    register: bool,
) -> Result<Envelope> {
    if register {
        // `register_user` returns `InvalidInput` on duplicate
        // username; surface that directly to the caller.
        services.identity.register_user(username, password).await?;
    }
    let issued = services
        .identity
        .authenticate_user(username, password)
        .await?;
    token::write(home, &issued)?;
    Ok(Envelope::ok_data(json!({ "token": issued.clone() })).with_text(issued))
}

/// `liquid auth whoami` — validate the active token and report
/// the principal it resolves to. Useful for shell scripts that
/// need to assert their own identity before mutating state.
pub async fn whoami(services: &CliServices, home: &Path) -> Result<Envelope> {
    let token_str = token::require(home)?;
    let principal = services.identity.validate_token(&token_str).await?;
    let kind = match principal {
        liquid_core::PrincipalId::User(_) => "user",
        liquid_core::PrincipalId::Agent(_) => "agent",
    };
    let principal_str = principal.to_string();
    let summary = principal_str.clone();
    Ok(Envelope::ok_data(json!({
        "principal": principal_str,
        "kind": kind,
    }))
    .with_text(summary))
}

fn parse_role(s: &str) -> Result<BuiltInRole> {
    match s {
        "WorkspaceOwner" => Ok(BuiltInRole::WorkspaceOwner),
        "WorkspaceMember" => Ok(BuiltInRole::WorkspaceMember),
        "AppViewer" => Ok(BuiltInRole::AppViewer),
        "AppEditor" => Ok(BuiltInRole::AppEditor),
        "Agent" => Ok(BuiltInRole::Agent),
        other => Err(LiquidError::InvalidInput(format!(
            "role not recognised: {other} (one of: \
             WorkspaceOwner, WorkspaceMember, AppViewer, AppEditor, Agent)"
        ))),
    }
}

/// Translate the optional `--scope <uuid>` arg into a
/// `Resource` matching the role's scope requirement. Phase-1
/// `AppViewer` / `AppEditor` are scoped on an `AppInstance` UUID;
/// every other role takes `None`.
fn parse_scope(raw: Option<&str>, role: BuiltInRole) -> Result<Option<Resource>> {
    let scope_required = matches!(role, BuiltInRole::AppViewer | BuiltInRole::AppEditor);
    match (raw, scope_required) {
        (Some(s), true) => {
            let uuid = Uuid::parse_str(s)
                .map_err(|e| LiquidError::InvalidInput(format!("scope not a uuid: {s}: {e}")))?;
            Ok(Some(Resource::AppInstance(liquid_core::AppInstanceId(
                uuid,
            ))))
        }
        (None, true) => Err(LiquidError::InvalidInput(format!(
            "role {role:?} requires --scope <app-instance-uuid>"
        ))),
        (Some(s), false) => Err(LiquidError::InvalidInput(format!(
            "role {role:?} does not take a scope (got --scope {s})"
        ))),
        (None, false) => Ok(None),
    }
}
