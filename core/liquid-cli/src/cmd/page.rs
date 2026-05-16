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

use crate::cmd::parse;
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

    let workspace = parse::workspace_id(workspace)?;
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

    let workspace = parse::workspace_id(workspace)?;
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

/// `liquid page history <path> --workspace <id>` — operation-log
/// entries that touched `<path>`, newest-first, NDJSON when
/// `--format json`. Same shape as `audit list` records but
/// filtered to one path (M7 / §5.8).
pub async fn history(
    services: &CliServices,
    home: &Path,
    user_path: &str,
    workspace: &str,
    limit: usize,
) -> Result<Envelope> {
    let caller_token = token::require(home)?;
    let principal = services.identity.validate_token(&caller_token).await?;
    let workspace = parse::workspace_id(workspace)?;
    let page_id = page_id_for(workspace, user_path);
    let perms = services.permissions.as_ref();
    require_permission!(perms, principal, Action::Read, Resource::Page(page_id));

    let store_path = parse_store_path(user_path)?;
    let log = services
        .store
        .operation_log(workspace, usize::max(limit, 1))
        .await?;
    let mut records: Vec<serde_json::Value> = Vec::new();
    for op in log {
        let matches_path = match &op.kind {
            liquid_vcs::OperationKind::Create { path, .. }
            | liquid_vcs::OperationKind::Update { path, .. }
            | liquid_vcs::OperationKind::Delete { path, .. } => {
                path.as_str() == store_path.as_str()
            }
            liquid_vcs::OperationKind::Undo { .. } => false,
        };
        if !matches_path {
            continue;
        }
        let action = match &op.kind {
            liquid_vcs::OperationKind::Create { .. } | liquid_vcs::OperationKind::Update { .. } => {
                "Write"
            }
            liquid_vcs::OperationKind::Delete { .. } => "Delete",
            liquid_vcs::OperationKind::Undo { .. } => "Undo",
        };
        let principal_str = match op.author {
            liquid_core::PrincipalId::User(u) => format!("u:{u}"),
            liquid_core::PrincipalId::Agent(a) => format!("a:{a}"),
        };
        records.push(serde_json::json!({
            "operation_id": op.id.to_string(),
            "commit_id": op.commit.to_string(),
            "timestamp_unix_millis": op.timestamp_unix_millis,
            "principal": principal_str,
            "action": action,
            "path": user_path,
            "message": op.message,
        }));
        if records.len() >= limit {
            break;
        }
    }
    Ok(Envelope::ok_records(records))
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

    let workspace = parse::workspace_id(workspace)?;
    let page_id = page_id_for(workspace, user_path);
    let op_id = parse::op_id(op)?;

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
