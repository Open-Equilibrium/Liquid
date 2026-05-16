# Liquid — Task Queue

Active and upcoming implementation tasks. One task per heading.
Use `.github/ISSUE_TEMPLATE/task.md` to create new tasks via GitHub Issues.

Agents: read the task carefully, check the referenced milestone in
`IMPLEMENTATION_PLAN.md`, then invoke the `implement` skill.

---

## Active tasks

### [TASK-002] `ContentStore` trait + `InMemoryContentStore`

**Phase:** 1
**Milestone:** M2 (IMPLEMENTATION_PLAN.md §5.2, sub-tasks 1 & 3 of 4)
**Status:** Done

**What.** Define the `ContentStore` trait in `liquid-vcs` (per §4.1) and ship
`InMemoryContentStore` — the test/dev backend that satisfies the trait without
any Jujutsu dependency. Includes a typed `Operation` log with `Create | Update
| Delete | Undo` variants and proper undo semantics. Trait error type
normalises to `LiquidError` (the §4.1 spec used `StoreError`; we reconcile to
the workspace-wide error type so cross-crate boundaries stay uniform).

**Acceptance criteria.**
- [x] `cargo test -p liquid-vcs` is green (12 tests incl. M2 plan-level criterion)
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` clean
- [x] No `unwrap()`/`expect()` outside `#[cfg(test)]`
- [x] `InMemoryContentStore` is `Send + Sync`
- [x] `IMPLEMENTATION_PLAN.md` §4.1 updated to reflect the trait signature actually shipped

### [TASK-003] `FilesystemContentStore` + ADR-001 (VCS persistence policy)

**Phase:** 1
**Milestone:** M2 (IMPLEMENTATION_PLAN.md §5.2, sub-task 2 — interim)
**Status:** Done
**Blocked by:** TASK-002

**What.** Ship an on-disk `ContentStore` implementation under
`<root>/<workspace_id>/` with atomic file writes (write-tmp + rename) and a
JSON-line operation log. ADR-001 captures the decision to defer the
`jj-lib`-backed `JujutsuContentStore` to TASK-004 (jj-lib's API is unstable;
proving the on-disk persistence path against the trait gets us the
operationally important property — durability — without committing to a
specific upstream version this session).

**Acceptance criteria.**
- [x] `cargo test -p liquid-vcs` is green for both InMemory and Filesystem stores (26 tests)
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` clean
- [x] No `unwrap()`/`expect()` outside `#[cfg(test)]`
- [x] Workspace data persists across `FilesystemContentStore` instances
- [x] Op log survives process restart (verified by re-opening the same root)
- [x] `docs/adr/001-jujutsu-pinning.md` accepted

### [TASK-004] `JujutsuContentStore` via `jj-lib`

**Phase:** 1
**Milestone:** M2 (IMPLEMENTATION_PLAN.md §5.2, sub-task 2 — final)
**Status:** Planned
**Blocked by:** TASK-003

**What.** Replace `FilesystemContentStore` with a thin wrapper over a real
Jujutsu workspace via the pinned `jj-lib` version named in ADR-001. The
trait abstraction (ADR-005) means callers won't change.

### [TASK-008] Minimal agent CLI (M6.5)

**Phase:** 1
**Milestone:** M6.5 (IMPLEMENTATION_PLAN.md §5.6)
**Status:** Planned
**Blocked by:** M5 FFI bridge

**What.** Ship the minimum `liquid` CLI surface that drives the MVP slice
(`tests/cli/00_mvp_slice.bats`): `workspace create`, `auth provision-agent`,
`auth token`, `page write`, `page read`, `audit list`, `page undo`. Every
command validates its token against `IdentityProvider` and runs
`require_permission!` before any state mutation (Absolute Rule 4). Output
follows the `--format text|json` convention from §12. Lands before any
Flutter shell work (M6) so the CLI-before-UI gate is unambiguous.

**Acceptance criteria.**
- [ ] `bats tests/cli/00_mvp_slice.bats` is green (the spec was added in
      the same commit train, currently `skip "pending M6.5"`).
- [ ] Every subcommand has a focused bats test covering the happy path
      and at least one auth-failure path.
- [ ] No `unwrap()` / `expect()` outside `#[cfg(test)]`.
- [ ] `IMPLEMENTATION_PLAN.md` §12 grammar matches every shipped subcommand.

### [TASK-009] Full agent CLI (M7)

**Phase:** 1
**Milestone:** M7 (IMPLEMENTATION_PLAN.md §5.8)
**Status:** Planned
**Blocked by:** TASK-008

**What.** Extend the CLI from M6.5 to cover the rest of §12: `workspace
list/delete`, `page history`, `auth login/whoami`, `app …` subcommands,
and the `--as` impersonation flag. Every mutation continues to run
`require_permission!` first; every command has bats coverage.

---

## Done tasks

### [TASK-007] Disk-backed `PermissionIndex`

**Phase:** 1
**Milestone:** M3 (IMPLEMENTATION_PLAN.md §5.3, last bullet)
**Status:** Done

**What.** Shipped `FilesystemPermissionIndex` — a TOML-backed
implementation of `PermissionIndex` persisting role bindings to
`<root>/workspaces/<id>/permissions.toml` (per §9). One file per
workspace, atomic writes via tmp-then-rename (same pattern as
`FilesystemContentStore` per ADR-001), in-memory cache keeping `check`
at the same complexity as the in-memory variant. The matching logic
moved into `Binding::matches()` so both backends share one definition.

**Acceptance criteria.**
- [x] `cargo test -p liquid-permissions` is green (12 in-memory unit +
      9 filesystem integration + 1 M3 end-to-end = 22 tests)
- [x] `cargo fmt --check` and
      `cargo clippy --workspace --all-targets --locked -- -D warnings`
      clean
- [x] No `unwrap()` / `expect()` outside test code
- [x] Bindings persist across instance restart
- [x] Workspace bindings stored in separate files; one workspace's
      permissions never load from another's file
- [x] Malformed TOML returns `LiquidError::InvalidInput`, never panics

### [TASK-005] `liquid-permissions` trait + `InMemoryPermissionIndex` + `require_permission!`

**Phase:** 1
**Milestone:** M3 (IMPLEMENTATION_PLAN.md §5.3, sub-tasks 2–4 of 4)
**Status:** Done

**What.** Define `PermissionIndex` (per §4.2 — updated to reflect Phase-1
scope: `BuiltInRole` enum instead of `RoleId`; `grant` deferred to Phase 3
along with custom roles). Ship `InMemoryPermissionIndex`, the
hard-coded `BuiltInRole` permission matrix
(`WorkspaceOwner | WorkspaceMember | AppViewer | AppEditor | Agent`), and
the `require_permission!` macro that gates every bridge / CLI callsite.

**Acceptance criteria.**
- [x] `cargo test -p liquid-permissions` is green (12 unit tests + the
      M3 plan-level end-to-end test that wires `liquid-auth` into the
      flow)
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`
      clean
- [x] No `unwrap()`/`expect()` outside `#[cfg(test)]`
- [x] Plan-level success criterion proven: AppViewer cannot write,
      AppEditor can, WorkspaceOwner can do both
- [x] §4.2 updated to reflect the trait shape actually shipped

### [TASK-006] `liquid-auth::LocalIdentityProvider`

**Phase:** 1
**Milestone:** M3 (IMPLEMENTATION_PLAN.md §5.3, sub-task 1 of 4)
**Status:** Done

**What.** Define `IdentityProvider` (per §4.5 — errors normalised to
`LiquidError`, `workspace_id` removed from token format) and ship
`LocalIdentityProvider`: TOML-backed users + agents (`users.toml`,
`agents.toml` under a configurable root), Argon2id password hashing,
HMAC-SHA256 session tokens of the form `principal . expires_unix .
hmac_hex`. `register_user` / `authenticate_user` live as inherent
helpers — they are local-only and Phase 3's OIDC backend will replace
them with a browser-redirect flow.

**Acceptance criteria.**
- [x] `cargo test -p liquid-auth` is green (13 integration tests
      including round-trip-across-restart and tampered/expired token
      rejection)
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`
      clean
- [x] No `unwrap()`/`expect()` outside `#[cfg(test)]`
- [x] Tokens reject tampering, wrong signing key, expiry, malformed input
- [x] Users and agents persist across provider restarts
- [x] §4.5 updated to reflect the trait shape actually shipped

### [TASK-001] Rust workspace bootstrap + `liquid-core` primitives

**Phase:** 1
**Milestone:** M1 (IMPLEMENTATION_PLAN.md §5.1)
**Status:** Done

**What.** Create the `core/` Cargo workspace with stubs for all eight crates
(`liquid-core`, `liquid-vcs`, `liquid-auth`, `liquid-permissions`,
`liquid-cache`, `liquid-bindings`, `liquid-sdk-bridge`, `liquid-cli`) and
fully implement `liquid-core`: ID newtypes, `PrincipalId`, `ContentHash`,
`StorePath`, `SlotName`, `SlotValue`, `Action`, `Resource`, `TenantConfig`,
`LiquidError`. Every public function returns `Result<_, LiquidError>`; no
`unwrap()`/`expect()` outside `#[cfg(test)]`.

**Acceptance criteria.**
- [x] `cargo test -p liquid-core` is green (26 tests)
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` clean
- [x] No `unwrap()`/`expect()` outside `#[cfg(test)]`
- [x] Every ID type has construction, equality, and serde round-trip tests
- [x] `StorePath` rejects `..`, absolute paths, empty segments
- [x] `SlotName` rejects malformed names
- [x] `ContentHash::from_hex` validates length and lowercase-hex
- [ ] CI's `detect` job picks up `core/Cargo.toml` and runs the rust matrix (verified post-push)

---

## Task template

Copy this block when adding a task directly to this file:

```markdown
## [TASK-NNN] Short title

**Phase:** 1 | 2 | 3 | 4
**Milestone:** M1–M20 (IMPLEMENTATION_PLAN.md reference)
**Status:** Planned | In Progress | Blocked | Done
**Blocked by:** TASK-NNN (if applicable)

### What
One paragraph describing the change and why it is needed.

### Acceptance criteria
- [ ] Failing test written and confirmed red
- [ ] Tests pass green
- [ ] CLI validates the feature end-to-end (bats tests/cli/)
- [ ] UI implemented (if applicable) with widget tests
- [ ] E2E integration test passes (if UI involved)
- [ ] Review pass clean (clippy, analyze, no unwrap, no platform imports)
- [ ] Docs updated (IMPLEMENTATION_PLAN.md, sdk-guide/, ADR if needed)

### Affected files
- `core/<crate>/src/`
- `app/lib/`
- `sdk/liquid_sdk/lib/`

### Notes
Any constraints, edge cases, or prior art worth knowing.
```
