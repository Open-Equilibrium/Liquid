# Liquid — Task Queue

Active and upcoming implementation tasks. One task per heading.
Use `.github/ISSUE_TEMPLATE/task.md` to create new tasks via GitHub Issues.

Agents: read the task carefully, check the referenced milestone in
`IMPLEMENTATION_PLAN.md`, then invoke the `implement` skill.

---

## Active tasks

### [TASK-004] `JujutsuContentStore` via `jj-lib`

**Phase:** 1
**Milestone:** M2 (IMPLEMENTATION_PLAN.md §5.2, sub-task 2 — final)
**Status:** Planned
**Blocked by:** TASK-003

**What.** Replace `FilesystemContentStore` with a thin wrapper over a real
Jujutsu workspace via the pinned `jj-lib` version named in ADR-001. The
trait abstraction (ADR-005) means callers won't change.

### [TASK-014] `liquid app …` subcommands (M7 follow-up, depends on M8)

**Phase:** 2
**Milestone:** M7 (IMPLEMENTATION_PLAN.md §5.8) — `app …` subset
**Status:** Planned
**Blocked by:** M8 — `AppManifest` + `ComponentManifest`

**What.** Implement the `app list / install / uninstall` +
`app <instance-name> read / write / slot subscribe / slot publish`
subcommands carved out of TASK-009. Each one needs the M8 SDK's
`AppManifest` + (for slot subcommands) M9's `SlotBroker`. Once
M8 ships, layer these onto the existing CLI dispatch table.

### [TASK-012] M5 Dart side — `flutter_rust_bridge` codegen + integration test

**Phase:** 1
**Milestone:** M5 (IMPLEMENTATION_PLAN.md §5.5, Dart half)
**Status:** Planned
**Blocked by:** M6 scaffolding `app/` + `sdk/liquid_sdk/`

**What.** Add `flutter_rust_bridge` to `liquid-sdk-bridge`, annotate
`BridgeServices` + the 5 entry points with `#[frb]`, run the codegen
into `app/lib/bridge/`, and write the Dart integration test the §5.5
success criterion describes (create workspace → write page → read
back → assert round-trip data + content_hash matches).

**Acceptance criteria.**
- [ ] `flutter test test/bridge_integration_test.dart` reports
      `+1: All tests passed!`
- [ ] `flutter_rust_bridge_codegen generate --no-write` produces
      output byte-identical to the committed `app/lib/bridge/*`
      files (codegen-version pin + no manual edits).
- [ ] Every `#[frb]`-annotated method on `BridgeServices` calls
      `IdentityProvider::validate_token` first and (for mutating /
      data-touching arms) `require_permission!` second (CLAUDE.md
      Absolute Rule 4 + ADR-004).
- [ ] `docs/manual-validation-m4-m5.md` §M5 STATUS flips from
      `RUST SIDE DONE; DART SIDE PENDING` to `DONE`; the §M5.4 +
      §M5.5 "PENDING TASK-012" tags are removed.

---

### [TASK-016b] M9 wiring UI on `PageGrid`

**Phase:** 2
**Milestone:** M9 — wiring UI half (`IMPLEMENTATION_PLAN.md §6.2`)
**Status:** Planned
**Blocked by:** M6 page tooling + TASK-012 (Dart bridge)

**What.** Add the long-press-on-output-slot → drag → drop-on-input
gesture to `PageGrid` that calls `bridge.wireSlots(...)` (TASK-012)
which translates to `liquid-bindings::InProcessSlotBroker::wire`
(TASK-016a, Done). Persists the resulting `BindingsDocument` to
`.liquid/pages/<page_id>/bindings.json` so wiring survives a page
reload.

### [TASK-017] M10 multi-instance tenant configuration

**Phase:** 2
**Milestone:** M10 (`IMPLEMENTATION_PLAN.md §6.3`)
**Status:** Planned
**Blocked by:** TASK-012 (M5 Dart side), TASK-011a (encryption helper)

**What.** AES-256-GCM-encrypted per-instance tenant config under
`.liquid/instances/<instance_id>/tenant.enc.json`; key derived
from the workspace owner's password via Argon2id (never stored
on disk). UI form generated from the app's
`TenantConfigSchema.jsonSchema` (already declared in the M8 SDK).

### [TASK-018] Re-enable multi-platform Flutter CI matrix

**Phase:** 4 (mobile + cross-platform polish)
**Milestone:** Pre-1.0 obligations checklist (`IMPLEMENTATION_PLAN.md §17`)
**Status:** Planned
**Blocked by:** Multi-platform scaffolding under `app/{android,ios,macos,windows}/`

**What.** M6 only requires the Flutter shell to launch on Linux
(`IMPLEMENTATION_PLAN.md §5.7`), so this branch's CI matrix in
`.github/workflows/ci.yml` ships with `target: linux` only.
When the project actually generates `flutter create --platforms=…`
scaffolding for Android, iOS, macOS, and Windows, restore the four
extra matrix entries (Android needs `android-actions/setup-android`,
iOS needs `--no-codesign`) and the per-platform `flutter build`
arms. Keep `dart format`, `flutter analyze`, and `flutter test`
linux-only so we don't pay 5× for tests that don't change per
platform.

## Done tasks

### [TASK-016a] M9 Rust side — `SlotBroker` + `InProcessSlotBroker`

**Phase:** 2
**Milestone:** M9 — Rust half (`IMPLEMENTATION_PLAN.md §6.2`)
**Status:** Done

**What.** Shipped the `liquid-bindings::SlotBroker` trait + the
`InProcessSlotBroker` Phase-2 backend (per-slot
`tokio::sync::broadcast` channels, in-memory wiring table,
JSON-serialisable `BindingsDocument` for page-reload replay).
Plus `SharedBroker` type alias (`Arc<dyn SlotBroker>`) ready for
the bridge to share across FFI workers.

**Acceptance criteria.**
- [x] `cargo test -p liquid-bindings` is green (9 inline tests).
- [x] `cargo clippy --workspace --all-targets --locked -- -D
      warnings` clean.
- [x] No `unwrap()` / `expect()` outside `#[cfg(test)]`.
- [x] `IMPLEMENTATION_PLAN.md §4.4` + §6.2 ticked for the Rust
      half; Dart side cross-referenced to TASK-012 + TASK-016b.

### [TASK-015] M8 Public Dart SDK API surface

**Phase:** 2
**Milestone:** M8 (`IMPLEMENTATION_PLAN.md §6.1`)
**Status:** Done

**What.** Scaffolded `sdk/liquid_sdk/` (`flutter create --template=package`)
and shipped the M8 API surface: `LiquidComponent`,
`InputSlot`/`OutputSlot`/`SlotSchema`, sealed
`SlotValue` with `when` matcher, `AppManifest`,
`ComponentManifest`, `Permission`, `TenantConfigSchema`,
`CliCommandDeclaration`, plus abstract `GridApi`/`VcsApi`/
`PermissionApi`/`SlotEmitter`/`SlotConsumer` runtime APIs. The
concrete `flutter_rust_bridge`-backed runtime impls ship with
TASK-012; the M8 SDK's job is the *typed surface developers
extend*.

**Acceptance criteria.**
- [x] `flutter test` is green (6 / 6 cases covering the M8
      plan-level success criterion — a `_ResetCounter` stub
      component declares one input + one output and exposes the
      typed surface).
- [x] `flutter analyze` clean.
- [x] `IMPLEMENTATION_PLAN.md §6.1` ticks every checkbox + the
      ones marked "abstract surface; concrete impl pending
      TASK-012".

### [TASK-013] M6 Flutter shell skeleton

**Phase:** 1 / 2 (transition)
**Milestone:** M6 (`IMPLEMENTATION_PLAN.md §5.7`)
**Status:** Done

**What.** Scaffolded `app/` (`flutter create --platforms=linux`)
and shipped the four canonical widgets: `RootShell` (resizable
`Row` of `ExplorerPanel` + `PageArea`), `ExplorerPanel`
(workspace switcher dropdown + placeholder section list),
`PageArea` (toolbar + `PageGrid`), `PageGrid` (12×12 grid,
drag-to-reposition, bottom-right resize handle, snap-to-grid).
Riverpod hosts every state container. One placeholder `GridItem`
seeds the grid so it's exercisable on first launch.

**Acceptance criteria.**
- [x] `flutter test` is green (4 / 4 widget tests covering the
      M6 success criterion: RootShell mounts the four widgets;
      workspace switcher lists demo workspaces; PageGrid hosts
      the placeholder; toolbar wires the documented affordances).
- [x] `flutter analyze` clean.
- [x] No `dart:io`, no platform plugins — Absolute Rule 2.
- [x] `IMPLEMENTATION_PLAN.md §5.7` ticks shipped checkboxes;
      the deeper `PageTreeView` / `AppInstanceListView` /
      `TagSectionView` items stay open as placeholder section
      headers (await M8 data sources).

### [TASK-009] Full agent CLI (M7)

**Phase:** 1
**Milestone:** M7 (IMPLEMENTATION_PLAN.md §5.8)
**Status:** Done — `app …` subset carved out to TASK-014 (Planned).

**What.** Shipped the remainder of the §12 CLI surface on top of
M6.5: `workspace list`, `workspace delete`, `page history`,
`auth login`, `auth whoami`, and the global `--as` impersonation
flag (accepts both bare-name lookup and principal-form ids).
Plus `BridgeServices::delete_workspace` (gated by
`Action::Admin`) + `WorkspaceRegistry::delete` (`InMemory` +
`Filesystem` variants) + `LocalIdentityProvider::find_agents_by_name`
+ `find_agent_by_principal` (drives the `--as` lookup).

**Acceptance criteria.**
- [x] `bats tests/cli/11_m7_full_cli.bats` is green (13 / 13 —
      workspace list / delete, page history, auth login / whoami,
      --as impersonation happy + negative paths).
- [x] Every mutating subcommand runs `require_permission!` first
      (directly or via the bridge's
      `delete_workspace` / `create_workspace` arms).
- [x] No `unwrap()` / `expect()` outside `#[cfg(test)]`.
- [x] `IMPLEMENTATION_PLAN.md §5.8` ticks every shipped checkbox;
      the `app …` rows are left unticked with a pointer to
      TASK-014.
- [x] `cargo clippy --workspace --all-targets --locked -- -D
      warnings` clean; `cargo fmt --all --check` clean.
- [x] Workspace delete is anti-enumeration: the permission check
      fires before the registry lookup so unknown workspaces
      surface as `Forbidden` rather than `NotFound` (§4.5).


### [TASK-008] Minimal agent CLI (M6.5)

**Phase:** 1
**Milestone:** M6.5 (IMPLEMENTATION_PLAN.md §5.6)
**Status:** Done

**What.** Shipped the seven §5.6 subcommands plus a
`FilesystemWorkspaceRegistry` (so workspace metadata survives
process restarts — the in-memory variant from TASK-011 is now the
test-only sibling). `BridgeServices` is composed at every CLI
invocation from `LocalIdentityProvider` + `FilesystemContentStore`
+ `FilesystemPermissionIndex` + `FilesystemWorkspaceRegistry`
rooted at `$LIQUID_HOME`; the first `workspace create` bootstraps a
default `cli` user + HMAC secret + bearer token under `$LIQUID_HOME`
so subsequent commands have a token to validate. Page-path is
mapped to `PageId` via `Uuid::new_v5(workspace_uuid, path_bytes)`
(stable per workspace, never collides across workspaces — §4.2
globally-unique-UUID assumption satisfied). The audit-list NDJSON
emit maps `OperationKind::{Create,Update}` to the user-visible
`Write` verb so the `--action Write` filter catches both.

**Acceptance criteria.**
- [x] `bats tests/cli/00_mvp_slice.bats` is green — 6/6 cases pass
      after dropping every `skip "pending M6.5"`.
- [x] Every subcommand has a focused bats test covering the happy
      path and at least one auth-failure / negative path
      (`tests/cli/10_cli_subcommands.bats`, 13 cases).
- [x] No `unwrap()` / `expect()` outside `#[cfg(test)]`.
- [x] `IMPLEMENTATION_PLAN.md §12` grammar matches every shipped
      subcommand; §5.6 ticks every checkbox; §9 `liquid-cli`
      describes the shipped state layout.
- [x] `cargo clippy --workspace --all-targets --locked -- -D
      warnings` clean; `cargo fmt --all --check` clean.
- [x] `.codecov.yml` keeps `core/liquid-cli/**` exempted per §15
      "≥ 80% line coverage on all crates except `liquid-cli`" —
      the CLI's behaviour test is bats, which tarpaulin does not
      see; the seven subcommands are covered by 19 bats cases.
- [x] Manual validation:
      [`docs/manual-validation-m6.5.md`](docs/manual-validation-m6.5.md).

### [TASK-011] M5 Rust side — `liquid-sdk-bridge` composition root + 5 FFI entry points

**Phase:** 1
**Milestone:** M5 (IMPLEMENTATION_PLAN.md §5.5, Rust half)
**Status:** Done

**What.** Shipped the Rust side of the M5 bridge:
`BridgeServices<S, P, I, R>` generic composition root over
`ContentStore` + `PermissionIndex` + `IdentityProvider` + the new
`WorkspaceRegistry`; five token-gated FFI entries on
`BridgeServices` (`create_workspace`, `list_workspaces`,
`load_page`, `write_page`, `check_permission`); `PageSnapshot` +
`WorkspaceSummary` wire types; `InMemoryWorkspaceRegistry` Phase-1
backend. ADR-004 records the adaptation from the §5.5 sketch's
free-standing `pub async fn` shape to the `BridgeServices`-with-
`token: &str` shape (the original signatures had no authentic
principal to gate against — Rule-4 violation).

**Acceptance criteria.**
- [x] `cargo test -p liquid-sdk-bridge` is green (5 inline unit +
      10 `m5_end_to_end` integration = 15 tests; covers every entry
      point, the tampered-token rejection path, the
      `Forbidden`-without-binding path, and the bytes +
      content-hash round-trip)
- [x] `cargo clippy --workspace --all-targets --locked -- -D
      warnings` clean
- [x] No `unwrap()` / `expect()` outside `#[cfg(test)]` /
      `#[allow(clippy::unwrap_used, …)]`-gated test mods
- [x] Every entry point validates the caller's token first;
      every mutating arm runs `require_permission!` second
      (`create_workspace` is the documented bootstrap exception
      per §9 + ADR-004)
- [x] `IMPLEMENTATION_PLAN.md` §5.5 (Rust side ticked, `[ ]`
      remaining for Dart side under TASK-012) and §9
      `liquid-sdk-bridge` entry updated to describe the shipped
      composition root + `WorkspaceRegistry` trait
- [x] `docs/manual-validation-m4-m5.md` §M5 STATUS flipped to
      `RUST SIDE DONE; DART SIDE PENDING (TASK-012)`; §M5.0–M5.3
      describe the Rust-side review steps



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

### [TASK-010] M4 cache layer stub — `ReadCache` + `InProcessCache` + `CachedContentStore`

**Phase:** 1
**Milestone:** M4 (IMPLEMENTATION_PLAN.md §5.4)
**Status:** Done

**What.** Shipped the Phase-1 cache layer: `liquid-cache::ReadCache`
trait + `InProcessCache` (`Arc<DashMap<ContentHash, Bytes>>`, no
expiry per §9) + `liquid-vcs::CachedContentStore<S, C>` wrapping
adapter that warms the cache on every `read` and invalidates the
prior `ContentHash` on every `write` / `undo`. The wrapper
maintains a `(WorkspaceId, StorePath) → ContentHash` index so the
second read of a path serves from the cache without touching the
inner store — the M4 plan-level success criterion. `undo`
conservatively invalidates every cached hash for the affected
workspace until TASK-004 (jj-lib backend) exposes per-op
affected-paths for a precise invalidation. `ContentHash::of_bytes`
helper added to `liquid-core` so SHA-256 hashing stays in one
place and Absolute Rule 1 is upheld in the cache call-sites.

**Acceptance criteria.**
- [x] `cargo test -p liquid-cache` is green (8 trait + impl tests)
- [x] `cargo test -p liquid-vcs --test cached_store` is green (7
      wiring tests, including the SpyStore-counter success criterion
      `second_read_of_same_path_is_served_from_cache`)
- [x] `cargo test -p liquid-core` is green (30 tests, +4 for
      `ContentHash::of_bytes` — RFC 6234 vectors, round-trip,
      collision-free)
- [x] `cargo fmt --check` and
      `cargo clippy --workspace --all-targets --locked -- -D warnings`
      clean
- [x] No `unwrap()` / `expect()` outside `#[cfg(test)]`
- [x] `IMPLEMENTATION_PLAN.md` §5.4 ticked and §4.3 trait shape
      matches the shipped `liquid-cache::ReadCache`
- [x] `deny.toml` `bans.skip` entry added for `hashbrown` (dashmap
      6.1 pins 0.14, toml 0.8 pulls 0.17 transitively; same shape
      as the existing `getrandom` skip)

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
- [x] `cargo test -p liquid-permissions` is green (14 in-memory unit
      + 9 filesystem integration + 4 filesystem-corners coverage +
      1 M3 end-to-end = 28 tests; the +2 in-memory tests
      *characterise* the §4.2 globally-unique-UUID tenant-isolation
      assumption — actual enforcement is `Uuid::new_v4()` in
      `AppInstanceId::new`; the +4 filesystem-corners tests cover
      open-time error paths)
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
- [x] `cargo test -p liquid-permissions` is green (14 unit tests + the
      M3 plan-level end-to-end test that wires `liquid-auth` into the
      flow; the +2 unit tests *characterise* the §4.2 globally-unique-
      UUID tenant-isolation assumption — the actual enforcement lives
      in `AppInstanceId::new` calling `Uuid::new_v4`, which no in-
      crate test can falsify)
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
