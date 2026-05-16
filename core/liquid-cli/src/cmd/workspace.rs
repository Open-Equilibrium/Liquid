//! `liquid workspace …` subcommands.

use std::path::Path;

use liquid_core::Result;
use serde_json::json;

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
