//! `liquid audit …` subcommands.
//!
//! Read-only view of `FilesystemContentStore`'s `op_log.jsonl`
//! per `IMPLEMENTATION_PLAN.md §12`. Filterable by principal /
//! action / since. NDJSON emit (`--format json`) is one operation
//! per line, oldest-first so a `tail -n 1` returns the newest.

use std::path::Path;

use liquid_auth::IdentityProvider;
use liquid_core::{Action, LiquidError, PrincipalId, Resource, Result, WorkspaceId};
use liquid_permissions::PermissionIndex;
use liquid_vcs::{ContentStore, Operation, OperationKind};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::output::Envelope;
use crate::services::CliServices;
use crate::token;

pub async fn list(
    services: &CliServices,
    home: &Path,
    workspace: &str,
    principal: Option<&str>,
    action: Option<&str>,
    since: Option<u64>,
    limit: usize,
) -> Result<Envelope> {
    let caller_token = token::require(home)?;
    let caller = services.identity.validate_token(&caller_token).await?;
    let workspace = parse_workspace_id(workspace)?;

    // Caller must hold Read on the workspace to inspect the audit log.
    let perms = services.permissions.as_ref();
    let allowed = perms
        .check(caller, Action::Read, Resource::Workspace(workspace))
        .await?;
    if !allowed {
        return Err(LiquidError::Forbidden);
    }

    // `operation_log` returns newest-first; we'll reverse to
    // oldest-first for NDJSON (so `tail -n 1` returns the newest).
    let log = services.store.operation_log(workspace, limit).await?;
    let filter_principal = principal.map(parse_principal).transpose()?;
    let filter_action = action.map(parse_action_filter).transpose()?;

    let mut rendered: Vec<Value> = log
        .iter()
        .filter(|op| match (&filter_principal, op.author) {
            (Some(want), got) => *want == got,
            (None, _) => true,
        })
        .filter(|op| match &filter_action {
            Some(want_action) => want_action.matches(&op.kind),
            None => true,
        })
        .filter(|op| match since {
            Some(min_unix) => (op.timestamp_unix_millis / 1000) >= min_unix,
            None => true,
        })
        .map(render_operation)
        .collect();

    // Newest is last per the spec's `tail -n 1` convention.
    rendered.reverse();

    Ok(Envelope::ok_records(rendered))
}

fn render_operation(op: &Operation) -> Value {
    // User-visible action mirrors the CLI verbs the user typed —
    // `page write` always renders as `Write` regardless of whether
    // the underlying op was a `Create` (first write of the path)
    // or an `Update` (overwrite). The op-log itself keeps the
    // distinction; the audit CLI surface flattens it for the
    // `--action Write` filter.
    let (action, path) = match &op.kind {
        OperationKind::Create { path, .. } | OperationKind::Update { path, .. } => {
            ("Write", path.as_str().to_string())
        }
        OperationKind::Delete { path, .. } => ("Delete", path.as_str().to_string()),
        OperationKind::Undo { target } => ("Undo", format!("op:{target}")),
    };
    let principal = match op.author {
        PrincipalId::User(u) => format!("u:{u}"),
        PrincipalId::Agent(a) => format!("a:{a}"),
    };
    let path = with_leading_slash(&path);
    json!({
        "operation_id": op.id.to_string(),
        "commit_id": op.commit.to_string(),
        "timestamp_unix_millis": op.timestamp_unix_millis,
        "principal": principal,
        "action": action,
        "path": path,
        "message": op.message,
    })
}

/// Re-add the leading `/` the CLI stripped when constructing the
/// `StorePath` so the audit-log output round-trips the user's
/// original `--path` style.
fn with_leading_slash(path: &str) -> String {
    if path.starts_with('/') || path.starts_with("op:") {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

/// Discriminated-union over the filter values the CLI accepts on
/// `--action`. `Write` is an alias matching both `Create` and
/// `Update` because the §12 grammar's `page write` emits one of
/// the two depending on whether the path existed.
enum ActionFilter {
    Create,
    Update,
    Delete,
    Undo,
    Write, // matches both Create + Update
}

impl ActionFilter {
    fn matches(&self, kind: &OperationKind) -> bool {
        matches!(
            (self, kind),
            (ActionFilter::Create, OperationKind::Create { .. })
                | (ActionFilter::Update, OperationKind::Update { .. })
                | (ActionFilter::Delete, OperationKind::Delete { .. })
                | (ActionFilter::Undo, OperationKind::Undo { .. })
                | (
                    ActionFilter::Write,
                    OperationKind::Create { .. } | OperationKind::Update { .. },
                )
        )
    }
}

fn parse_action_filter(s: &str) -> Result<ActionFilter> {
    match s {
        "Create" => Ok(ActionFilter::Create),
        "Update" => Ok(ActionFilter::Update),
        "Delete" => Ok(ActionFilter::Delete),
        "Undo" => Ok(ActionFilter::Undo),
        "Write" => Ok(ActionFilter::Write),
        other => Err(LiquidError::InvalidInput(format!(
            "action filter not recognised: {other} (one of Create, Update, Delete, Undo, Write)"
        ))),
    }
}

fn parse_workspace_id(s: &str) -> Result<WorkspaceId> {
    Uuid::parse_str(s)
        .map(WorkspaceId)
        .map_err(|e| LiquidError::InvalidInput(format!("workspace id not a uuid: {s}: {e}")))
}

fn parse_principal(s: &str) -> Result<PrincipalId> {
    let (kind, id) = s
        .split_once(':')
        .ok_or_else(|| LiquidError::InvalidInput(format!("principal id missing prefix: {s}")))?;
    let uuid = Uuid::parse_str(id)
        .map_err(|e| LiquidError::InvalidInput(format!("principal id not a uuid: {s}: {e}")))?;
    match kind {
        "u" | "user" => Ok(PrincipalId::User(uuid)),
        "a" | "agent" => Ok(PrincipalId::Agent(uuid)),
        other => Err(LiquidError::InvalidInput(format!(
            "principal kind not recognised: {other}"
        ))),
    }
}
