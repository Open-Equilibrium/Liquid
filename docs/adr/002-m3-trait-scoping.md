# ADR-002 — M3 trait shape: hard-coded roles, no `grant`, no workspace-bound tokens

**Status:** Accepted
**Date:** 2026-05-05
**Deciders:** Claude (M3 implementer), repository maintainers

## Context

`IMPLEMENTATION_PLAN.md` §4.2 (`PermissionIndex`) and §4.5
(`IdentityProvider`) were drafted as forward-looking traits that mix
Phase-1 needs with Phase-3 needs. M3 (§5.3) is explicit that:

- "Built-in roles … is hard-coded in phase 1; configurable in phase 3" (§9).
- The Phase-1 backend is in-memory; the disk-backed variant is queued (§5.3).
- "Token = HMAC-SHA256-signed `{principal_id, workspace_id, expires_at}` blob"
  (§5.3); §9 elaborates the format.

When implementing M3, three of the originally-drafted trait elements turn
out to be either dead or actively misleading in Phase 1, and need a
decision:

1. `PermissionIndex::grant(role, action, resource)` — only meaningful when
   roles have configurable permission sets, which Phase 1 does not.
2. `assign_role(... role: RoleId)` — `RoleId(Uuid)` is opaque; the
   built-in roles need stable identifiers, and the only callers of
   `assign_role` in Phase 1 want to name a built-in role.
3. The `workspace_id` field inside the session token. The token represents
   the principal's identity, not their authority over a workspace. A token
   does not, on its own, decide what the principal may do — that is
   `PermissionIndex`'s job. Carrying `workspace_id` in the token invites
   future code to treat the field as authorisation, which would silently
   bypass `PermissionIndex`.

We need a defensible shape for the M3 traits that ships now without
foreclosing the Phase-3 design (custom roles, OIDC, multi-workspace
sessions).

## Decision

For Phase 1 we ship the trait shapes actually exercised by the Phase-1
code, and document the deferred surface in §4.2 / §4.5:

**`PermissionIndex`:**

```rust
async fn check(&self, principal, action, resource) -> Result<bool>;
async fn assign_role(&self, workspace, principal, role: BuiltInRole, scope: Option<Resource>) -> Result<()>;
async fn revoke_role(&self, workspace, principal, role: BuiltInRole, scope: Option<Resource>) -> Result<()>;
```

- `RoleId` → `BuiltInRole` enum
  (`WorkspaceOwner | WorkspaceMember | AppViewer | AppEditor | Agent`).
- `grant` is removed; the role → permission matrix is encoded in
  `BuiltInRole::permits`. Phase 3 will reintroduce `grant` alongside a
  `RoleId` variant on `BuiltInRole` (or sibling enum) when custom roles
  ship.
- `assign_role` gains `scope: Option<Resource>`. `AppViewer` /
  `AppEditor` require a non-`None` scope; workspace-wide roles take
  `None`.
- All errors normalise to `LiquidError` (consistent with §4.1 / §4.5).

**`IdentityProvider`:**

```rust
async fn validate_token(&self, token: &str) -> Result<PrincipalId>;
async fn issue_token(&self, principal: PrincipalId) -> Result<String>;
async fn provision_agent(&self, workspace, authorized_by, name) -> Result<PrincipalId>;
```

- Errors normalise to `LiquidError`. Any auth failure becomes
  `LiquidError::Forbidden`; we never leak the failure mode.
- Token format becomes
  `principal . expires_unix . hmac_hex`
  (three dot-separated, URL-safe-by-construction fields).
  `principal` is `u:<uuid>` for users, `a:<uuid>` for agents.
- The `workspace_id` field from the §9 draft is dropped.

## Rationale

**Trait-as-shipped is honest about what's wired up.** Stubbing `grant`
to return `Err(LiquidError::Forbidden)` would compile, but it would let
callers write code that silently never works in Phase 1 and surprises
in production. Removing `grant` from the trait makes the unsupported
operation un-callable; `BuiltInRole::permits` is the matrix, full stop.

**`BuiltInRole` is type-safe.** `RoleId(Uuid)` is opaque — Phase-1
callers would need a side-table mapping built-in names to UUIDs, and
that table would be a place to make mistakes. An enum makes "WorkspaceOwner"
the same value at every callsite by construction.

**Scope is required by the matrix.** `AppViewer` and `AppEditor` are
defined per §9 to grant access to *a specific* app instance — a binding
without a scope is ill-formed. Pushing the scope into the binding
(rather than implicit in the role) makes the data model match the
semantics, and makes "owner of workspace A" / "viewer of app X" both
expressible in the same shape.

**Workspace-bound tokens are a footgun.** A session token answers
"who are you?". A permission check answers "may you do X?". Mixing
them in the token invites future code to skip the permission check on
the assumption that the token's `workspace_id` already authorised the
operation. That code path doesn't exist today; we keep it from being
written by removing the field.

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Ship `grant` as `Err(Forbidden)` stub | Misleading: callers can write code that compiles and never works. Worse than not having the method. |
| Hard-code stable UUIDs for built-in roles | Replaces a typed enum with a magic-number table. Adds a place for typos and a translation step at every callsite. No upside in Phase 1. |
| Keep `RoleId` and accept opaque keys | Loses type safety on the only callers Phase 1 has. Adds a lookup table. |
| Keep `workspace_id` in the token but document it as informational | Documentation does not stop the next implementer from treating the field as authorisation. The bug class (skipping `PermissionIndex` because the token "already says" it's authorised) is severe enough to be designed out, not commented around. |
| Defer M3 entirely until Phase-3 trait shape is settled | Blocks M5 (FFI) and M7 (CLI). The Phase-1 shape we need is a strict subset of the eventual shape; "ship the subset, document the deferral" is the same playbook as ADR-001. |

## Consequences

**Easier:**
- Phase-1 callers (M5 bridge, M7 CLI) get a small, type-safe surface.
  `require_permission!(index, principal, action, resource)` is the only
  permission gate they need.
- The Phase-3 trait extensions (custom roles, OIDC) can be additive:
  introduce a `Role` enum with `BuiltIn(BuiltInRole) | Custom(RoleId)`,
  re-introduce `grant`, and the existing call sites keep compiling.

**Harder:**
- A future caller who wants to grant a built-in role a *different*
  permission than the matrix encodes cannot do it without dropping into
  the trait extension that Phase 3 will add. That is intentional.
- TASK-007 (disk-backed `PermissionIndex`) inherits the same trait
  shape; the TOML format will need to encode `BuiltInRole` and
  `Option<Resource>` rather than two UUIDs. This is straightforward but
  worth noting up front.

**Existing code / rules affected:**
- `IMPLEMENTATION_PLAN.md` §4.2 and §4.5 are updated to match the
  shipped trait shapes.
- CLAUDE.md rule 4 ("Permission check is always first") is satisfied by
  the `require_permission!` macro — bridge and CLI authors call it as
  the first line of every entrypoint.
