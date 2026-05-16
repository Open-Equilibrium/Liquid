//! `liquid workspace …` subcommands.

use std::path::Path;

use liquid_core::Result;
use serde_json::{json, Value};

use crate::cmd::parse;
use crate::output::Envelope;
use crate::services::CliServices;
use crate::token;

/// `liquid workspace create <name>` — bootstraps a default `cli`
/// user + token under `$LIQUID_HOME` on first run, then calls
/// `BridgeServices::create_workspace`.
pub async fn create(services: &CliServices, home: &Path, name: String) -> Result<Envelope> {
    let token_str = match token::resolve(home) {
        Some(t) => t,
        None => token::bootstrap(services, home).await?,
    };
    let workspace = services.create_workspace(&token_str, name.clone()).await?;
    let summary = format!("created workspace {workspace} (name: {name})");
    Ok(Envelope::ok_data(json!({
        "workspace_id": workspace.to_string(),
        "name": name,
    }))
    .with_text(summary))
}

/// `liquid workspace list` — NDJSON one record per workspace the
/// caller has at least Read authority on. Bootstraps if no token is
/// on disk so a fresh `$LIQUID_HOME` produces an empty list rather
/// than a "no token" error (matches §5.6's bootstrap exception for
/// `workspace create`).
pub async fn list(services: &CliServices, home: &Path) -> Result<Envelope> {
    let token_str = match token::resolve(home) {
        Some(t) => t,
        None => token::bootstrap(services, home).await?,
    };
    let summaries = services.list_workspaces(&token_str).await?;
    let records: Vec<Value> = summaries
        .into_iter()
        .map(|s| {
            json!({
                "workspace_id": s.id.to_string(),
                "name": s.name,
                "created_by": s.created_by.to_string(),
                "created_unix": s.created_unix,
            })
        })
        .collect();
    Ok(Envelope::ok_records(records))
}

/// `liquid workspace delete <id>` — requires `Action::Admin` on
/// the workspace. Does NOT cascade-delete the on-disk VCS bytes
/// (see `BridgeServices::delete_workspace`).
pub async fn delete(services: &CliServices, home: &Path, id: &str) -> Result<Envelope> {
    let token_str = token::require(home)?;
    let workspace = parse::workspace_id(id)?;
    services.delete_workspace(&token_str, workspace).await?;
    let summary = format!("deleted workspace {workspace}");
    Ok(Envelope::ok_data(json!({
        "workspace_id": workspace.to_string(),
    }))
    .with_text(summary))
}
