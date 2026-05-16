//! Shared per-subcommand argument parsers.
//!
//! Extracted from the cmd handlers to dedupe the identical
//! `Uuid::parse_str` → `WorkspaceId` mapping the audit / auth /
//! page modules each used (CLAUDE.md anti-redundancy rule).

use liquid_core::{LiquidError, OperationId, Result, WorkspaceId};
use uuid::Uuid;

/// `--workspace <uuid>` → typed `WorkspaceId`. The error message
/// names the offending input so an agent can fix its template.
pub fn workspace_id(s: &str) -> Result<WorkspaceId> {
    Uuid::parse_str(s)
        .map(WorkspaceId)
        .map_err(|e| LiquidError::InvalidInput(format!("workspace id not a uuid: {s}: {e}")))
}

/// `--op <uuid>` → typed `OperationId`. Same error shape as
/// [`workspace_id`].
pub fn op_id(s: &str) -> Result<OperationId> {
    Uuid::parse_str(s)
        .map(OperationId)
        .map_err(|e| LiquidError::InvalidInput(format!("operation id not a uuid: {s}: {e}")))
}
