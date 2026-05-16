//! `liquid page …` subcommands.
//!
//! The CLI is the permission-enforcement point for its own calls
//! (`IMPLEMENTATION_PLAN.md §5.6`): every command validates the
//! token via `IdentityProvider::validate_token`, then runs
//! `require_permission!` against the resolved principal before
//! delegating to `ContentStore`.
//!
//! Path convention: the user-supplied `<path>` (e.g.
//! `/pages/welcome`) is normalised to a workspace-relative
//! `StorePath` by stripping the leading `/`. The corresponding
//! `Resource::Page` id is derived from `(workspace, path)` via
//! UUID v5 so the same path always maps to the same `PageId`
//! within a workspace, and never collides across workspaces
//! (satisfying the §4.2 globally-unique-UUID assumption — the
//! namespace is the workspace UUID, so distinct workspaces yield
//! distinct page IDs even for identical paths).

use std::path::Path;

use bytes::Bytes;
use liquid_auth::IdentityProvider;
use liquid_core::{Action, LiquidError, PageId, Resource, Result, StorePath, WorkspaceId};
use liquid_permissions::require_permission;
use liquid_vcs::ContentStore;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::output::Envelope;
use crate::services::CliServices;
use crate::token;

pub async fn write(
    services: &CliServices,
    home: &Path,
    user_path: &str,
    workspace: &str,
    data: Option<String>,
    file: Option<String>,
    message: String,
) -> Result<Envelope> {
    let caller_token = token::require(home)?;
    let principal = services.identity.validate_token(&caller_token).await?;

    let workspace = parse_workspace_id(workspace)?;
    let store_path = parse_store_path(user_path)?;
    let page_id = page_id_for(workspace, user_path);

    let perms = services.permissions.as_ref();
    require_permission!(perms, principal, Action::Write, Resource::Page(page_id));

    let bytes = resolve_bytes(data, file)?;
    let commit = services
        .store
        .write(workspace, &store_path, bytes, principal, &message)
        .await?;

    // We need the OperationId of the write the bats spec asserts on
    // — `op_log.first()` returns the newest entry, which is the
    // mutation we just performed.
    let log = services.store.operation_log(workspace, 1).await?;
    let op_id = log
        .first()
        .map_or_else(|| "<missing>".to_string(), |op| op.id.to_string());

    let summary = format!("wrote {user_path} as commit {commit}");
    Ok(Envelope::ok_data(json!({
        "path": user_path,
        "commit_id": commit.to_string(),
        "operation_id": op_id,
    }))
    .with_text(summary))
}

pub async fn read(
    services: &CliServices,
    home: &Path,
    user_path: &str,
    workspace: &str,
) -> Result<Envelope> {
    let caller_token = token::require(home)?;
    let principal = services.identity.validate_token(&caller_token).await?;

    let workspace = parse_workspace_id(workspace)?;
    let store_path = parse_store_path(user_path)?;
    let page_id = page_id_for(workspace, user_path);

    let perms = services.permissions.as_ref();
    require_permission!(perms, principal, Action::Read, Resource::Page(page_id));

    let bytes = services.store.read(workspace, &store_path).await?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|e| {
        LiquidError::InvalidInput(format!(
            "page at {user_path} is not valid JSON: {e} (raw bytes: {} bytes)",
            bytes.len()
        ))
    })?;

    Ok(Envelope::ok_data(value))
}

pub async fn undo(
    services: &CliServices,
    home: &Path,
    user_path: &str,
    workspace: &str,
    op: &str,
) -> Result<Envelope> {
    let caller_token = token::require(home)?;
    let principal = services.identity.validate_token(&caller_token).await?;

    let workspace = parse_workspace_id(workspace)?;
    let page_id = page_id_for(workspace, user_path);
    let op_id = parse_op_id(op)?;

    let perms = services.permissions.as_ref();
    require_permission!(perms, principal, Action::Write, Resource::Page(page_id));

    let commit = services.store.undo(workspace, op_id).await?;
    let summary = format!("undone op {op_id} on {user_path} (synthetic commit {commit})");
    Ok(Envelope::ok_data(json!({
        "path": user_path,
        "operation_id": op_id.to_string(),
        "commit_id": commit.to_string(),
    }))
    .with_text(summary))
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn parse_workspace_id(s: &str) -> Result<WorkspaceId> {
    Uuid::parse_str(s)
        .map(WorkspaceId)
        .map_err(|e| LiquidError::InvalidInput(format!("workspace id not a uuid: {s}: {e}")))
}

fn parse_op_id(s: &str) -> Result<liquid_core::OperationId> {
    Uuid::parse_str(s)
        .map(liquid_core::OperationId)
        .map_err(|e| LiquidError::InvalidInput(format!("operation id not a uuid: {s}: {e}")))
}

/// Strip the leading `/` so the user-visible `/pages/welcome`
/// becomes the workspace-relative `pages/welcome` that `StorePath`
/// requires.
fn parse_store_path(s: &str) -> Result<StorePath> {
    let trimmed = s.strip_prefix('/').unwrap_or(s);
    StorePath::new(trimmed)
}

/// Derive a stable per-workspace `PageId` from the user path via
/// UUID v5. The namespace is the workspace's own UUID — same path
/// in a different workspace yields a different `PageId`, so the
/// §4.2 globally-unique-UUID tenant-isolation assumption is
/// satisfied by construction.
fn page_id_for(workspace: WorkspaceId, user_path: &str) -> PageId {
    PageId(Uuid::new_v5(&workspace.0, user_path.as_bytes()))
}

/// Read the bytes payload from `--data <inline-json>` or
/// `--file <path>`. clap already enforces mutual exclusion + at
/// least one of the two via `conflicts_with`.
fn resolve_bytes(data: Option<String>, file: Option<String>) -> Result<Bytes> {
    match (data, file) {
        (Some(d), None) => Ok(Bytes::from(d.into_bytes())),
        (None, Some(path)) => {
            let bytes = std::fs::read(&path)
                .map_err(|e| LiquidError::InvalidInput(format!("read --file {path}: {e}")))?;
            Ok(Bytes::from(bytes))
        }
        (None, None) => Err(LiquidError::InvalidInput(
            "either --data <json> or --file <path> is required".into(),
        )),
        // clap's conflicts_with prevents this; defensive.
        (Some(_), Some(_)) => Err(LiquidError::InvalidInput(
            "--data and --file are mutually exclusive".into(),
        )),
    }
}
