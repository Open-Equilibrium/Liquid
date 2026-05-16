# ADR-004 — `liquid-sdk-bridge` takes `token: &str` as the first FFI argument

**Status:** Accepted
**Date:** 2026-05-16
**Deciders:** Liquid Phase-1 maintainers (M5 — TASK-011 review)

> Numbering note: this is the next *tactical disk ADR* after
> `001-jujutsu-pinning.md`, `002-m3-trait-scoping.md`,
> `003-oss-policy.md`. It is independent of the strategic ADR-NNN
> labels in `IMPLEMENTATION_PLAN.md §15` — those go up to ADR-008
> on a separate numbering track. See the §15 numbering note for the
> convention.

## Context

`IMPLEMENTATION_PLAN.md §5.5` originally sketched the M5 FFI surface as five
top-level `pub async fn` items:

```rust
pub async fn create_workspace(name: String) -> Result<WorkspaceId>;
pub async fn list_workspaces(principal: String) -> Result<Vec<WorkspaceSummary>>;
pub async fn load_page(workspace: WorkspaceId, page_id: PageId) -> Result<PageSnapshot>;
pub async fn write_page(workspace: WorkspaceId, page_id: PageId,
                        snapshot: PageSnapshot, author: String,
                        message: String) -> Result<CommitId>;
pub async fn check_permission(principal: String, action: String,
                              resource: String) -> Result<bool>;
```

Three signatures (`create_workspace`, `load_page`, `write_page`) carry no
principal at all; two (`list_workspaces`, `check_permission`) carry a
`principal: String` that the caller could trivially spoof. Both shapes
contradict `CLAUDE.md`'s Absolute Rule 4 ("permission check is always
first") because there is no authentic principal to gate against.

The bridge also needs a place to bind every backend (`ContentStore`,
`PermissionIndex`, `IdentityProvider`, the new `WorkspaceRegistry`).
Free-standing `pub async fn` items require those backends to live in
process-global state — testable only via cargo-test setup hooks,
swappable only via cargo feature flags.

## Decision

The five M5 entry points become inherent `async` methods on a generic
`BridgeServices<S, P, I, R>` struct that takes the four backends as
`Arc<...>` fields. Every method takes `token: &str` as its first
argument and validates it via `IdentityProvider::validate_token`
before any other logic.

```rust
impl<S, P, I, R> BridgeServices<S, P, I, R>
where S: ContentStore, P: PermissionIndex, I: IdentityProvider, R: WorkspaceRegistry
{
    pub async fn create_workspace(&self, token: &str, name: String) -> Result<WorkspaceId>;
    pub async fn list_workspaces(&self, token: &str) -> Result<Vec<WorkspaceSummary>>;
    pub async fn load_page(&self, token: &str, workspace: WorkspaceId, page_id: PageId)
                           -> Result<PageSnapshot>;
    pub async fn write_page(&self, token: &str, workspace: WorkspaceId, page_id: PageId,
                            snapshot: PageSnapshot, message: String) -> Result<CommitId>;
    pub async fn check_permission(&self, token: &str, principal: &str,
                                  action: Action, resource: Resource) -> Result<bool>;
}
```

`flutter_rust_bridge` codegen (TASK-012) will then emit Dart methods on a
matching `BridgeServices` class whose constructor receives the host-side
service handles; every Dart call passes the active session token as the
first argument.

The `author: String` argument of the original `write_page` is dropped —
the author is sourced from the validated token to prevent impersonation
(a caller cannot claim to be someone else; the token IS the identity).

## Rationale

- **Authenticity:** a `token: &str` is unforgeable (HMAC-signed per §4.5).
  A `principal: String` is plaintext input. Rule 4 demands the former
  at every bridge boundary.
- **Testability:** a struct-with-`Arc`-fields lets the M5 end-to-end
  test (`core/liquid-sdk-bridge/tests/m5_end_to_end.rs`) wire in
  `InMemoryContentStore + InMemoryPermissionIndex + LocalIdentityProvider
  + InMemoryWorkspaceRegistry` per scenario, without any test-only feature
  flag or global state.
- **Composability:** generic over the four trait shapes ⇒ production
  swaps (`FilesystemContentStore` + `FilesystemPermissionIndex`) cost a
  type substitution at the composition root, not a call-site edit.
- **No business logic leak:** the bridge still does only what §5.5 / §9
  promised — token validation, permission gating, registry insert,
  store delegation. The `WorkspaceRegistry` trait is the one new
  abstraction; it is the only concept the spec required but had not
  yet been named.

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Keep the literal §5.5 signatures (`principal: String` parameter) | Violates Absolute Rule 4. A bridge call with no token is unauthenticatable; a `principal: String` parameter is spoofable. |
| Store the token in `tokio::task_local!` / ambient context | Same composability story as the chosen design but adds a hidden global that complicates testing and reasoning about call origin. The struct field is explicit. |
| Free-standing `pub async fn` + lazy-initialised global services | Requires `OnceCell<BridgeServices>` + careful setup-ordering. Tests would need either feature flags to override or a tear-down convention. The struct-with-`&self` design removes the entire class of "did the test set it up?" bugs. |
| Drop `write_page`'s `author: String` *and* expose a separate `impersonate(...)` method | Out of scope for Phase 1. Impersonation in CLI lands in M7 (§5.8) via the `--as <agent-name>` flag, which still uses the calling agent's token + an additional permission check. |

## Consequences

**Easier:**
- Phase 3's Redis / OIDC backend swap is a type substitution at the
  composition root.
- Adding a sixth bridge method does not require a `tokio::task_local!`
  ceremony — just a new `impl BridgeServices` method.
- Every bridge call has the same first-argument shape, so codegen +
  reviewers learn the convention once.

**Harder:**
- The §5.5 signatures in `IMPLEMENTATION_PLAN.md` no longer match the
  Rust source verbatim. The §5.5 text now describes the actually-shipped
  shape; readers comparing to old `git log` will see the adaptation.
- Dart-side TASK-012 must thread the token through every call;
  `flutter_rust_bridge` codegen will produce matching positional args.

**Existing rules unchanged:**
- Absolute Rule 4 (permission check first) — strengthened, not weakened.
- Absolute Rule 5 (every storage call carries `WorkspaceId`) — every
  bridge method that touches the store passes `workspace: WorkspaceId`
  through to `ContentStore`.

**See also:**
- `IMPLEMENTATION_PLAN.md §5.5` — milestone description (updated).
- `IMPLEMENTATION_PLAN.md §9` `liquid-sdk-bridge` entry — describes
  the `BridgeServices` composition root + the `WorkspaceRegistry` trait.
- `core/liquid-sdk-bridge/tests/m5_end_to_end.rs` — the 10-scenario
  end-to-end test that proves the contract.
- `CLAUDE.md` "Absolute Rules" §4 — the rule this ADR honours.
