//! The 5 entry-point FFI functions described in
//! `IMPLEMENTATION_PLAN.md §5.5`.
//!
//! Each function:
//!   1. Validates the caller's token via `IdentityProvider::validate_token`
//!      (collapsing every auth failure to `LiquidError::Forbidden` per
//!      `IMPLEMENTATION_PLAN.md §4.5`).
//!   2. For mutating / data-touching calls, runs `require_permission!`
//!      against the resolved principal (`CLAUDE.md` Absolute Rule 4 —
//!      "Permission check is always first").
//!   3. Delegates to the relevant backend (`ContentStore`,
//!      `PermissionIndex`, `WorkspaceRegistry`). No business logic
//!      lives in this module beyond the gate.
//!
//! Phase-1 sequencing of mutations:
//!
//!   - `create_workspace` does NOT call `require_permission!`. Workspace
//!     creation is a bootstrap operation; the caller has no binding
//!     yet (binding is created as a side-effect). Token validation
//!     gates the call.
//!   - `list_workspaces` is workspace-scoped, so the permission gate
//!     is per-row (filter to workspaces the principal has at least one
//!     binding in) rather than at the call boundary.
//!   - `load_page`, `write_page` use `require_permission!` against
//!     `Resource::Page(page_id)`.
//!   - `check_permission` exposes `PermissionIndex::check` — it
//!     authenticates the caller (so a tampered token cannot run the
//!     query) but does not gate the query subject.

use bytes::Bytes;
use liquid_auth::IdentityProvider;
use liquid_core::{
    Action, CommitId, LiquidError, PageId, PrincipalId, Resource, Result, StorePath, WorkspaceId,
};
use liquid_permissions::{require_permission, BuiltInRole, PermissionIndex};
use liquid_vcs::ContentStore;

use crate::registry::{WorkspaceRecord, WorkspaceRegistry};
use crate::services::BridgeServices;
use crate::types::{PageSnapshot, WorkspaceSummary};

/// `pages/<page_id>` — the convention the bridge uses to map a `PageId`
/// to a `StorePath` for `ContentStore` calls. Centralised here so the
/// `load_page` and `write_page` arms cannot drift.
fn page_path(page_id: PageId) -> Result<StorePath> {
    StorePath::new(format!("pages/{}", page_id.0.simple()))
}

/// `PrincipalId::Display` produces `"user:<uuid>"` / `"agent:<uuid>"`;
/// this parses the same shape back. Used by `check_permission` to
/// resolve the query subject's principal id from the wire string.
fn parse_principal(s: &str) -> Result<PrincipalId> {
    let (kind, id) = s
        .split_once(':')
        .ok_or_else(|| LiquidError::InvalidInput(format!("principal id missing prefix: {s}")))?;
    let uuid = uuid::Uuid::parse_str(id)
        .map_err(|e| LiquidError::InvalidInput(format!("principal id not a uuid: {s}: {e}")))?;
    match kind {
        "user" => Ok(PrincipalId::User(uuid)),
        "agent" => Ok(PrincipalId::Agent(uuid)),
        other => Err(LiquidError::InvalidInput(format!(
            "principal id kind not recognised: {other}"
        ))),
    }
}

/// Build the `InvalidInput` error returned when a `write_page` call's
/// `snapshot.page_id` does not match the call's positional `page_id`.
/// Pulled out of the call site so the format string lives on one
/// instrumented line (codecov stops flagging the multi-line format!
/// args as uncovered).
fn page_id_mismatch(actual: PageId, expected: PageId) -> LiquidError {
    LiquidError::InvalidInput(format!(
        "snapshot.page_id ({actual}) does not match call's page_id ({expected})"
    ))
}

/// Seconds since the Unix epoch, sourced from `SystemTime::now()`.
///
/// Returns `0` if the system clock is set before 1970 (e.g. an
/// uninitialised RTC). The fallback honours Absolute Rule 1 (no
/// `unwrap` / `expect` in production code) but does have a known
/// degraded-sort consequence: every workspace created during a
/// clock-skew window will share `created_unix = 0` and therefore
/// sort to the back of `list_workspaces` results. This is preferable
/// to panicking the bridge entry point — a misordered list is a
/// degraded UX, a panic across the FFI boundary corrupts state.
fn now_unix() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl<S, P, I, R> BridgeServices<S, P, I, R>
where
    S: ContentStore,
    P: PermissionIndex,
    I: IdentityProvider,
    R: WorkspaceRegistry,
{
    /// Create a new workspace owned by the authenticated caller.
    /// Records `{id, name, created_by, created_unix}` in the registry
    /// and assigns the caller the `WorkspaceOwner` role binding via the
    /// active `PermissionIndex`.
    ///
    /// Phase-1: any authenticated principal can create a workspace.
    /// Phase 3 will add an admin / quota gate.
    pub async fn create_workspace(&self, token: &str, name: String) -> Result<WorkspaceId> {
        let principal = self.identity.validate_token(token).await?;
        if name.trim().is_empty() {
            return Err(LiquidError::InvalidInput("workspace name is empty".into()));
        }
        let id = WorkspaceId::new();
        self.registry
            .register(WorkspaceRecord {
                id,
                name,
                created_by: principal,
                created_unix: now_unix(),
            })
            .await?;
        // Owner role binding — workspace-scope (no `scope` arg) per
        // `IMPLEMENTATION_PLAN.md §9` Built-in roles table.
        self.permissions
            .assign_role(id, principal, BuiltInRole::WorkspaceOwner, None)
            .await?;
        Ok(id)
    }

    /// Every workspace the authenticated caller has at least Read
    /// authority over. Order: newest-first by `created_unix`
    /// (`WorkspaceRegistry::list` sorts already).
    pub async fn list_workspaces(&self, token: &str) -> Result<Vec<WorkspaceSummary>> {
        let principal = self.identity.validate_token(token).await?;
        let all = self.registry.list().await?;
        let perms = self.permissions.as_ref();
        let mut visible = Vec::with_capacity(all.len());
        for summary in all {
            let res = Resource::Workspace(summary.id);
            if perms.check(principal, Action::Read, res).await? {
                visible.push(summary);
            }
        }
        Ok(visible)
    }

    /// Read the current bytes of `page_id` in `workspace`. Returns a
    /// snapshot whose `content_hash` is derived from the bytes by
    /// `PageSnapshot::new`, so a Dart caller can compare hashes
    /// without rehashing.
    pub async fn load_page(
        &self,
        token: &str,
        workspace: WorkspaceId,
        page_id: PageId,
    ) -> Result<PageSnapshot> {
        let principal = self.identity.validate_token(token).await?;
        let perms = self.permissions.as_ref();
        require_permission!(perms, principal, Action::Read, Resource::Page(page_id));
        let path = page_path(page_id)?;
        let bytes = self.store.read(workspace, &path).await?;
        Ok(PageSnapshot::new(page_id, bytes))
    }

    /// Atomic write of `snapshot.bytes` at the canonical `pages/<id>`
    /// path inside `workspace`. The mutation is attributed to the
    /// authenticated caller (the §5.5 spec carries an `author: String`
    /// argument, but Phase-1 sources the author from the validated
    /// token to prevent impersonation).
    pub async fn write_page(
        &self,
        token: &str,
        workspace: WorkspaceId,
        page_id: PageId,
        snapshot: PageSnapshot,
        message: String,
    ) -> Result<CommitId> {
        let principal = self.identity.validate_token(token).await?;
        let perms = self.permissions.as_ref();
        require_permission!(perms, principal, Action::Write, Resource::Page(page_id));
        if snapshot.page_id != page_id {
            return Err(page_id_mismatch(snapshot.page_id, page_id));
        }
        let path = page_path(page_id)?;
        let bytes: Bytes = snapshot.bytes;
        self.store
            .write(workspace, &path, bytes, principal, &message)
            .await
    }

    /// Authenticate the caller and surface the underlying
    /// `PermissionIndex::check` result for the supplied query subject.
    /// The query subject's principal id arrives as a string
    /// (`"user:<uuid>"` / `"agent:<uuid>"`) to keep the FFI surface
    /// flat for Dart codegen.
    pub async fn check_permission(
        &self,
        token: &str,
        principal: &str,
        action: Action,
        resource: Resource,
    ) -> Result<bool> {
        // Authenticate the caller — a tampered token cannot run a
        // permission query, even though the query itself is read-only.
        let _caller = self.identity.validate_token(token).await?;
        let subject = parse_principal(principal)?;
        self.permissions.check(subject, action, resource).await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn parse_principal_round_trips_user_and_agent() {
        let u = PrincipalId::new_user();
        let a = PrincipalId::new_agent();
        assert_eq!(parse_principal(&u.to_string()).unwrap(), u);
        assert_eq!(parse_principal(&a.to_string()).unwrap(), a);
    }

    #[test]
    fn parse_principal_rejects_unknown_kind() {
        let err = parse_principal("bot:00000000-0000-0000-0000-000000000000").unwrap_err();
        assert!(matches!(err, LiquidError::InvalidInput(_)));
    }

    #[test]
    fn parse_principal_rejects_missing_colon() {
        let err = parse_principal("not-a-principal").unwrap_err();
        assert!(matches!(err, LiquidError::InvalidInput(_)));
    }

    #[test]
    fn parse_principal_rejects_bad_uuid() {
        let err = parse_principal("user:not-a-uuid").unwrap_err();
        assert!(matches!(err, LiquidError::InvalidInput(_)));
    }

    #[test]
    fn page_path_uses_pages_prefix_and_simple_uuid() {
        let id = PageId(uuid::Uuid::nil());
        let path = page_path(id).unwrap();
        assert_eq!(path.as_str(), "pages/00000000000000000000000000000000");
    }
}
