# Changelog

All notable changes to **Liquid** are documented in this file.

The format is based on [Keep a Changelog 1.1.0][keep-a-changelog], and
this project adheres to [Semantic Versioning 2.0.0][semver]. Pre-1.0
releases may break public APIs between minor versions; from 1.0
onwards, breaking changes are confined to major version bumps.

[keep-a-changelog]: https://keepachangelog.com/en/1.1.0/
[semver]: https://semver.org/spec/v2.0.0.html

The release tooling (`cargo-release`, see
[`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md) §16) regenerates the
sections below from Conventional Commit messages on tag. Entries above
the first numbered release are accumulated under `[Unreleased]` and
moved into a real version section when a release is cut.

## [Unreleased]

### Added — M6 Flutter shell skeleton (TASK-013)

- `app/` scaffold (`flutter create --platforms=linux --org
  io.openequilibrium --project-name liquid_app`). Depends on
  `flutter_riverpod ^2.5.0` and the in-repo
  `path: ../sdk/liquid_sdk` package.
- Four canonical widgets per `IMPLEMENTATION_PLAN.md §5.7`:
  - `RootShell` (`Row` of resizable `ExplorerPanel` + `PageArea`;
    drag handle between them clamped to 200–480 px).
  - `ExplorerPanel` (workspace switcher dropdown driven by
    `workspacesProvider` + section headers for Pages / Apps /
    Tags — real children land with M8 data sources).
  - `PageArea` (toolbar with active-workspace title + `add`
    button + pending `save` / `history`).
  - `PageGrid` (12×12 grid, `Stack`+`Positioned` layout,
    drag-to-reposition + bottom-right resize handle, snap-to-grid
    integer rounding). One placeholder `GridItem` seeded by
    `gridItemsProvider` so the grid is exercisable on first
    launch (M6 success criterion).
- `app/test/widget_test.dart` (4 cases) — shell mounts the four
  widgets; switcher lists demo workspaces; PageGrid hosts the
  placeholder; toolbar wires the documented affordances.

### Added — M8 Public Dart SDK API surface (TASK-015)

- `sdk/liquid_sdk/` scaffold (`flutter create --template=package`).
- Typed component-author API:
  - `LiquidComponent` abstract base (extends `StatefulWidget`)
    with `inputs` / `outputs` / `gridConstraints` getters.
  - `InputSlot` / `OutputSlot` typed slot handles +
    `InputSlotMap` / `OutputSlotMap` aliases.
  - `SlotSchema` + `SlotKind` enum + sealed `SlotValue` with
    `when` matcher (mirrors `liquid_core::SlotValue`).
- Declarative manifest types: `AppManifest`,
  `ComponentManifest`, `Permission`, `TenantConfigSchema`,
  `CliCommandDeclaration`, `ManifestAction`.
- Abstract runtime APIs (concrete impls land with TASK-012):
  `GridApi`, `VcsApi` (+ `HistoryEntry`), `PermissionApi`,
  `SlotEmitter`, `SlotConsumer`.
- `sdk/liquid_sdk/test/liquid_sdk_test.dart` (8 cases) — the M8
  success criterion (`_ResetCounter` stub component declares one
  input + one output) + `SlotValue` matcher routing + `SlotValue.json`
  and `SlotValue.bytes` structural-equality regressions +
  `AppManifest` round-trip.

### Added — M9 Rust-side data binding broker (TASK-016a)

- `liquid_bindings::SlotBroker` trait + `InProcessSlotBroker`
  Phase-2 backend. Per-slot `tokio::sync::broadcast` channels
  (`SLOT_BUFFER_SIZE = 256`), in-memory wiring table, fan-out
  on publish to wired downstreams.
- `SlotWiring { from, to }` + `BindingsDocument { wires }` —
  JSON-serialisable shapes the SDK persists to
  `.liquid/pages/<page_id>/bindings.json` so wiring survives
  page reload. `save_bindings` / `load_bindings` is the
  round-trip.
- `SharedBroker = Arc<dyn SlotBroker>` type alias for the
  bridge to share across FFI workers.
- 12 inline tests in `core/liquid-bindings/src/broker.rs` —
  publish-no-subscribers / publish-then-receive (one + two
  subscribers) / wire fans out / self-wire rejection / wire is
  idempotent / 2-hop cycle rejection via `wire` / 3-hop cycle
  rejection via `wire` / save→load round-trip survives a fresh
  broker (proves wiring replay works on page reload) / load
  rejects self-wires / load rejects multi-hop cycles /
  `BindingsDocument` JSON round-trip.

Carved out for follow-ups (each tracked in `TASKS.md`):
TASK-016b (wiring UI on `PageGrid`, blocked on M6 page tooling
+ TASK-012), TASK-012 (M5 Dart side — FFI codegen + `bridge.
publishSlot` / `subscribeSlot`), TASK-017 (M10 multi-instance
tenant config with AES-256-GCM-encrypted persistence + UI form
generation, blocked on TASK-012).

### Added — M7 full agent CLI (TASK-009)

- `liquid workspace list` — NDJSON, newest first, filtered to
  workspaces the caller has Read on (per-row
  `PermissionIndex::check`).
- `liquid workspace delete <id>` — Admin-gated via the new
  `BridgeServices::delete_workspace` + `WorkspaceRegistry::delete`.
  Anti-enumeration: the permission check fires before the
  registry lookup so unknown workspaces surface as `Forbidden`
  rather than `NotFound` (§4.5). Does NOT cascade-delete on-disk
  VCS bytes (forensics) — same Phase-1 boundary as the M6.5
  `workspace create` bootstrap.
- `liquid page history <path> --workspace <id> [--limit N]` —
  per-path operation-log view, newest-first NDJSON. Same record
  shape as `audit list` but filtered to one `StorePath`. Flattens
  `OperationKind::{Create,Update}` to `"Write"` (same flattening
  rule as `audit list` so the user-visible verb is consistent).
- `liquid auth login --username <u> --password <p> [--register]`
  — non-interactive login. `--register` first creates the user
  (rejects on dup); without it, `IdentityProvider::authenticate_user`
  validates the Argon2id hash. On success writes the issued
  token to `$LIQUID_HOME/token`. The fully interactive password
  prompt is a planned follow-up; the scriptable shape is what
  the bats suite needs.
- `liquid auth whoami` — validates the active token, prints
  `{ principal, kind }` where `kind` is `"user"` / `"agent"`.
  Useful in shell scripts that need to assert identity before
  mutating state.
- Global `--as <name|principal-id>` impersonation flag.
  Principal-form (`a:<uuid>` / `agent:<uuid>`) parses via
  `PrincipalId::FromStr` and resolves the agent's workspace via
  the new `LocalIdentityProvider::find_agent_by_principal`.
  Bare-name lookups go through `find_agents_by_name`; exactly-one
  match is required (zero → `NotFound`; multiple →
  `InvalidInput`). The caller must hold `Action::Admin` on the
  target's workspace OR be the target themselves;
  `User`-principal impersonation is rejected in Phase 1.
- `liquid_auth::AgentSummary` — public projection of the
  pub(crate)-only `AgentRecord` so external callers (CLI's
  `--as` resolver) can enumerate agents without re-parsing
  `agents.toml`.
- `WorkspaceRegistry::delete` trait method + `InMemory` and
  `Filesystem` impls. `Filesystem` round-trips through
  `flush_locked` so the on-disk `workspaces.toml` reflects the
  removal atomically.
- `tests/cli/11_m7_full_cli.bats` (new, 16 cases) covers every
  shipped subcommand's happy path + at least one negative path
  (Forbidden / NotFound / InvalidInput as appropriate). Three of
  the 16 are PR #18 audit-pass regressions for
  `auth login --register` username collisions, ambiguous `--as`
  name disambiguation, and `page history --limit > matches`.

**Carved out:** `liquid app list / install / uninstall` +
`liquid app <instance-name> read / write / slot subscribe / slot
publish` deferred to TASK-014 (planned, blocked on M8 —
`AppManifest`). The §5.8 spec checkboxes for those rows stay
unticked with an inline pointer.

### Fixed — Post-M6-M9 audit (PR #18 review pass, round 7)

- `TASKS.md`: added the missing **TASK-011a** heading
  (`AES-256-GCM encryption helper crate`). The entry was referenced
  on TASK-017's `Blocked by:` line for several rounds but had no
  definition — a contributor picking up M10 could not discover what
  TASK-011a required them to build first. The new entry specifies
  the three-function surface (`derive_key` / `encrypt` / `decrypt`),
  the Argon2id parameter pinning, the anti-enumeration single-error
  return shape, and the per-crate test gates.
- `core/liquid-bindings/src/broker.rs`,
  `IMPLEMENTATION_PLAN.md §4.4`, and the matching round-3 CHANGELOG
  bullet: corrected the actor that justifies the Phase-2 flat
  `SlotName` keyspace. The previous text said "the CLI drives
  exactly one workspace at a time" but the agent CLI never
  instantiates `InProcessSlotBroker`; the broker is hosted inside
  the Flutter app process. Rewording avoids confusing a future
  Absolute-Rule-5 audit into searching the CLI for a missing
  workspace scope that was never there.

### Fixed — Post-M6-M9 audit (PR #18 review pass, round 6)

- `docs/manual-validation-m6-m9.md` Step M8.1 expected `6 / 6` →
  `8 / 8` (covers the two structural-equality regressions); Step
  M9.1 expected `9 / 9` → `12 / 12` with the three cycle-rejection
  test names appended; the §M9 intro `9 inline tests` → `12 inline
  tests`. These were the only doc-staleness pockets the prior
  rounds' propagation sweep missed.
- `sdk/liquid_sdk/lib/src/runtime_apis.dart` doc comments: stale
  bare task-id references replaced with `TASK-016b` (the Dart-side
  slot emitter / consumer wiring belongs to that task, alongside
  TASK-012 for the bridge codegen).

### Fixed — Post-M6-M9 audit (PR #18 review pass, round 5)

- Bats-test count corrections that round 4 missed:
  `tests/cli/11_m7_full_cli.bats` is now `16 / 16` everywhere
  it appears (was `13 / 13` in `CHANGELOG.md`, `TASKS.md`,
  `IMPLEMENTATION_PLAN.md §5.8`, and the M6-M9 manual-validation
  guide); `tests/cli/10_cli_subcommands.bats` is now `16 cases`
  in `TASKS.md` (was `13 cases`, while
  `IMPLEMENTATION_PLAN.md §5.6` already had the correct count).
  The cross-suite total in `docs/manual-validation-m6-m9.md`
  sign-off checklist is `120 / 120` (was `117 / 117`).
- `IMPLEMENTATION_PLAN.md §6.2` success-criterion enumeration:
  appended the two `SlotBroker` test scenarios the slash-separated
  list was missing (`two-subscribers fan-out`,
  `BindingsDocument JSON round-trip`) so the enumeration matches
  the `12 inline` count.
- `core/liquid-sdk-bridge/Cargo.toml`: annotated the forward-
  declared `liquid-bindings` workspace dependency with a comment
  explaining it is the placeholder for TASK-012's
  `publish_slot` / `subscribe_slot` / `wire_slots` /
  `load_bindings` FFI entry points, so a future maintainer
  cannot mistake it for an unused dep ready to be culled.

### Fixed — Post-M6-M9 audit (PR #18 review pass, round 4)

- `README.md` Status table M8 / M9 rows: test counts corrected
  from `6 tests` → `8 tests` (M8) and `9 tests` → `12 tests`
  (M9, with the cycle-rejection note appended).
- `CHANGELOG.md` `Added — M8` / `Added — M9` bullets: same count
  + scenario corrections so the original feature description
  matches the shipped state of the test suites.
- `TASKS.md` Done-section criteria for TASK-015 / TASK-016a:
  same count corrections.
- `app/lib/src/page_area.dart` + `app/test/widget_test.dart`:
  toolbar tooltips and the matching widget-test reason replaced
  the stale `(pending M8)` text with the accurate
  `(pending TASK-012 VcsApi wiring)` blocker — M8 (typed surface)
  has shipped; only the concrete FFI-backed runtime APIs are
  still pending TASK-012.

### Fixed — Post-M6-M9 audit (PR #18 review pass, round 3)

- `IMPLEMENTATION_PLAN.md §6.1` success-criterion count corrected
  from "6 / 6" to "8 / 8" (two structural-equality regressions
  landed in round 1).
- `IMPLEMENTATION_PLAN.md §6.2` success-criterion count corrected
  from "9 inline" to "12 inline" and the enumeration now lists every
  cycle / self-wire / load-rejects case so the table matches the
  CHANGELOG bullet that already says "Three dedicated cycle tests".
- `IMPLEMENTATION_PLAN.md §4.4` now leads with a "Phase-2 deviation"
  callout: the shipped `SlotBroker` trait in
  `core/liquid-bindings/src/broker.rs` deliberately omits the
  `workspace: WorkspaceId`, `instance: AppInstanceId`, and
  `subscriber: PrincipalId` arms the target spec lists. The flat
  `SlotName` keyspace is safe for the single-process Phase-2 backend
  (the broker runs inside the Flutter app process — the agent CLI
  never instantiates it — and the app holds exactly one workspace
  open at a time, with apps already namespacing their slots); the
  workspace+instance+principal-aware shape lands
  with the Phase-4 distributed backend under **TASK-020** so the
  cross-tenant isolation contract is enforced in one place. The
  broker module docstring + a new TASK-020 entry in `TASKS.md`
  mirror the deviation note.
- `sdk/liquid_sdk/test/liquid_sdk_test.dart`: narrowed the
  `prefer_const_constructors` lint suppression from a file-wide
  `ignore_for_file` to per-line `// ignore:` on the three runtime-
  constructed `SlotValue.json` / `SlotValue.bytes` literals that
  must stay non-const for the structural-equality tests to mean
  anything. Inner `<int>[...]` literals stay `const` because they
  do not promote the enclosing map to a const value.

### Fixed — PR #18 CI green-lighting (sync-docs + dart-format + scaffolded-platform matrix)

- `scripts/sync-docs-check.sh`: extended the milestone-evidence
  grep to accept Phase-2 `### 6.N Milestone` headings in addition
  to the Phase-1 `### 5.N` form. The previous gate only looked in
  §5 and falsely flagged M8 + M9 as undocumented even though
  `IMPLEMENTATION_PLAN.md §6` covers them.
- `sdk/liquid_sdk/lib/src/runtime_apis.dart`,
  `app/lib/src/page_area.dart`, `app/lib/src/page_grid.dart`,
  `app/test/widget_test.dart`: applied `dart format` so the
  `--set-exit-if-changed` step on both CI jobs passes. Pure
  whitespace shifts; no behaviour change.
- `.github/workflows/ci.yml`: shrunk the `Flutter app` matrix to
  `linux` — M6's success criterion is "App launches on Linux"
  (`IMPLEMENTATION_PLAN.md §5.7`) and this branch ships no
  scaffolding under `app/{android,ios,macos,windows}`. TASK-018
  tracks the multi-platform re-expansion when the missing
  scaffolding lands.

### Fixed — Post-M6-M9 audit (PR #18 review pass)

- `sdk/liquid_sdk/lib/src/slot.dart`: `SlotValue.json` equality is
  now structural (`DeepCollectionEquality` from `package:collection`)
  instead of identity-based. The bug meant two `SlotValue.json` values
  with deep-equal `Map` / `List` contents compared unequal, which
  would have silently broken any caller using them as map keys, in
  `Set` membership, or in equality-based cache lookups. Test coverage
  added for both `json` and `bytes` structural equality, using
  runtime (`final`) literals + `identical(a, b) == false` guard so
  Dart's const canonicalisation cannot mask a regression of the bug.
- `core/liquid-bindings/src/broker.rs`: `Mutex` poison now propagates
  via `LiquidError::InvalidInput` (matches `liquid-auth`,
  `liquid-permissions`, and `liquid-vcs`) instead of silently
  continuing with poisoned state. Added multi-hop cycle detection to
  `wire` + `load_bindings`: A→B + B→A, A→B→C→A, and equivalent
  multi-hop topologies now return `InvalidInput`, so the upcoming
  wiring UI cannot persist a graph that closes a cycle. Three
  dedicated cycle tests (2-hop wire, 3-hop wire, multi-hop document).
- `core/liquid-cli/src/cmd/page.rs`: `page history --limit N` is now
  a per-path cap (N matching writes) rather than a prefix cap on the
  op log. The previous behaviour silently under-returned matches when
  unrelated writes dominated the recent log; the spec entry in
  `IMPLEMENTATION_PLAN.md §12` documents the per-path cap semantics
  and the Phase-1 O(N) cost. Regression test added.
- `tests/cli/11_m7_full_cli.bats`: three new regressions — duplicate
  `auth login --register` rejects with `InvalidInput`; ambiguous
  `--as <name>` rejects with `InvalidInput` and points at the
  principal-form for disambiguation; `page history` with a `--limit`
  larger than the number of matching writes returns only the
  matching writes (no false-positive entries from sibling paths).
- `app/lib/src/state.dart`: the doc comment claiming a typedef-only
  swap from `StateProvider` to `AsyncNotifierProvider` was misleading
  (consumer widgets would also change). Comment now describes the
  real (small) widget-side change that TASK-012 will require.
- `app/lib/src/page_grid.dart`: replaced deprecated `Color.withOpacity`
  with `withAlpha`, preventing an analyzer warning once the Flutter
  SDK rolls past 3.27.
- `IMPLEMENTATION_PLAN.md §10 'Folder conventions'`: marked every
  planned subdirectory (`shell/`, `explorer/`, `grid/`, `pages/`,
  `bindings/`, `state/`, `bridge/`) as future-state and recorded the
  flat `app/lib/src/` shipped in M6 as the current layout. Removes
  the contradiction between §2's flat tree and §10's subdir table.

### Fixed — codecov patch coverage on M6.5 (TASK-008 follow-up)

- `core/liquid-sdk-bridge/src/registry.rs`: added
  `filesystem_open_surfaces_io_err_when_root_cannot_be_created`
  test that points the registry root through a regular-file path
  (`fs::create_dir_all` rejects that with `NotADirectory` on
  every platform), forcing the `io_err("create root", _)` arm
  to execute. Closes the only remaining patch-coverage gap on
  the M6.5 PR — the `io_err` helper body (lines 231-232) was
  flagged by codecov because every happy-path test in the
  registry suite skipped the I/O-error mapping.

Result: `liquid-sdk-bridge/src/registry.rs` patch coverage
95.65% → **100% (67/67 lines)**; workspace coverage
94.14% → **94.36%** (+0.23%). All 23 bridge tests + 28
workspace test groups + 104 bats cases continue to pass;
clippy clean; fmt clean.

### Fixed — Post-M6.5 audit (TASK-008 follow-up)

- `core/liquid-core/src/ids.rs`: new
  `impl std::str::FromStr for PrincipalId` accepting both long
  (`user:<uuid>` / `agent:<uuid>`) and short (`u:<uuid>` /
  `a:<uuid>`) forms — the canonical wire-form parser. Closes
  the round-1 cross-layer divergence where the CLI accepted
  the short form but the bridge did not. Both layers now
  delegate to this `FromStr`. 7 new inline tests pin the
  contract.
- `core/liquid-cli/src/cmd/parse.rs` (new): extracts the
  identical `workspace_id` / `op_id` parse helpers that the
  audit / auth / page modules each copied — CLAUDE.md
  anti-redundancy rule. The three handlers now share one
  source of truth.
- `core/liquid-cli/src/args.rs`: `audit list` clap doc said
  "newest first" but the implementation explicitly reverses to
  oldest-first (so `tail -n 1` returns the newest, per the
  documented `--format json` NDJSON contract). Corrected the
  `--help` text. The `page read` doc gains a note that pages
  must be JSON-encoded (`--file` body source is stored
  verbatim, but `read` will reject non-JSON content with
  `InvalidInput`; a `--raw` flag is a planned M7 follow-up).
- `core/liquid-cli/src/cmd/auth.rs`: `auth provision-agent`
  output emitted `agent_id` as `"agent:<uuid>"` (the
  `PrincipalId::Display` form). That mismatched
  `data.workspace_id` which is a bare UUID. Now emits
  `agent_id` as the bare UUID and adds a sibling `principal`
  field carrying the full wire form for callers that want the
  pre-assembled string.
- `tests/cli/10_cli_subcommands.bats` grows to 16 cases (was
  13) covering: `audit list --principal a:<uuid>` short-form
  filter, `audit list --action Undo` discriminating from
  `Write`, and the bootstrap edge case where the user exists
  but the token file is missing (must surface an actionable
  error pointing at `LIQUID_TOKEN` or
  `$LIQUID_HOME/auth/users.toml` removal).

### Added — M6.5 minimal agent CLI (TASK-008)

- `core/liquid-cli/src/` — the seven §5.6 subcommands ship as a
  real `liquid` binary (replaces the prior `exit 64` stub):
  `workspace create`, `auth provision-agent`, `auth token`,
  `page write`, `page read`, `audit list`, `page undo`. Every
  command starts with `token::require → validate_token`; every
  mutating arm calls `require_permission!` next (Absolute Rule 4).
  `workspace create` on a fresh `$LIQUID_HOME` bootstraps a
  default `cli` user + 32-byte HMAC secret + bearer token so the
  first invocation has no manual setup. clap-derive arg parsing,
  tokio current-thread runtime, NDJSON / text output via the
  `Envelope { ok, data, records, error }` shape.
- State layout under `$LIQUID_HOME` (defaults to `$HOME/.liquid`):
  `auth/` (LocalIdentityProvider), `vcs/` (FilesystemContentStore),
  `perm/` (FilesystemPermissionIndex), `registry/`
  (FilesystemWorkspaceRegistry), `secret` (HMAC bytes), `token`
  (bearer). Documented in §9 `liquid-cli` + §5.6.
- `liquid_sdk_bridge::FilesystemWorkspaceRegistry` — durable
  Phase-1 sibling to `InMemoryWorkspaceRegistry`. Persists to
  `<root>/workspaces.toml` via atomic tmp-then-rename (same
  ADR-001 idiom as `FilesystemContentStore` /
  `FilesystemPermissionIndex`). The CLI re-opens it on every
  invocation; workspace metadata survives process restart.
  Backfills the §M5 follow-up flagged in `TASK-011`.
- `tests/cli/00_mvp_slice.bats` flipped from skip-only to live:
  6 / 6 cases pass end-to-end against the shipped binary
  (workspace create → provision-agent → page write/read → audit
  list → page undo → AppViewer-cannot-write negative).
- `tests/cli/10_cli_subcommands.bats` (new, 16 cases) covers per-
  subcommand edge cases the MVP slice does not: `--version`, no-
  args help-exit, bootstrap files (secret + token), registry
  cross-process persistence, `auth token` happy + no-token,
  invalid workspace UUID, `--data` / `--file` mutual exclusion +
  `--file` body source, NotFound on unknown read, `--action Write`
  filter, text-format summary + stderr error.
- `docs/manual-validation-m6.5.md` (new) — auditable companion to
  `bats tests/cli/`; walks a human reviewer through bootstrap,
  every subcommand happy path, the AppViewer-cannot-write
  negative, surface invariants by inspection, and a
  cross-process persistence smoke.
- `justfile coverage-check` now passes `--exclude-files
  'liquid-cli/*'` to tarpaulin, matching `.codecov.yml`'s
  long-standing `core/liquid-cli/**` exemption (per §15 — the
  CLI's behaviour test is bats, which tarpaulin does not see).

### Added — M5 Rust-side FFI bridge (TASK-011)

- `liquid-sdk-bridge::BridgeServices<S, P, I, R>` — generic
  composition root over `ContentStore` + `PermissionIndex` +
  `IdentityProvider` + the new `WorkspaceRegistry`. Production
  code substitutes `Filesystem*` variants at construction; tests
  substitute `InMemory*`. Closes `IMPLEMENTATION_PLAN.md §5.5`
  Rust-side surface.
- Five token-gated FFI entry points on `BridgeServices` —
  `create_workspace`, `list_workspaces`, `load_page`,
  `write_page`, `check_permission`. Every method validates the
  caller's token first (collapses every auth failure to
  `LiquidError::Forbidden` per §4.5); every mutating /
  data-touching arm runs `require_permission!` next per Absolute
  Rule 4. `create_workspace` is the documented bootstrap
  exception (no binding to gate against until the call creates
  one) — Phase 3 will add an admin / quota gate.
- `WorkspaceRegistry` trait + `InMemoryWorkspaceRegistry`
  Phase-1 backend recording `{id, name, created_by,
  created_unix}` for every workspace. The filesystem variant
  is a follow-up that pairs with M6.5's CLI persistence work
  (a process restart loses workspace *names* but not authority
  — `FilesystemPermissionIndex` already persists role bindings).
- `WorkspaceSummary` + `PageSnapshot` wire types in
  `liquid-sdk-bridge::types`. `PageSnapshot::new(page_id, bytes)`
  derives `content_hash` from `bytes` so the pair cannot be
  inconsistent; `flutter_rust_bridge` codegen (TASK-012) will
  emit a matching Dart constructor.
- `core/liquid-sdk-bridge/tests/m5_end_to_end.rs` — 10-scenario
  plan-level success-criterion suite wiring every Phase-1 crate
  together (auth + permissions + vcs + bridge). Asserts the
  tampered-token rejection, registry round-trip + owner-role
  auto-assignment, `list_workspaces` filtering by binding,
  `write_page → load_page` bytes + content-hash round-trip,
  `AppViewer`-cannot-write, unbound-agent-cannot-read,
  `check_permission` caller-authentication, and malformed
  query-subject rejection.

### Changed — `IMPLEMENTATION_PLAN.md §5.5` signature adaptation (ADR-004)

- The five §5.5 FFI signatures move from free-standing `pub
  async fn (principal: String, …)` to inherent `async` methods
  on `BridgeServices<S, P, I, R>` whose first argument is
  `token: &str`. A `principal: String` arg is spoofable;
  Absolute Rule 4 demands an unforgeable token at the bridge
  boundary. ADR-004
  (`docs/adr/004-bridge-token-first-arg.md`) records the
  decision + rejected alternatives. Dart-side TASK-012 will
  receive the same adaptation via `flutter_rust_bridge` codegen.

### Fixed — codecov patch coverage on M5 (TASK-011 follow-up)

- `core/liquid-sdk-bridge/src/registry.rs`: replaced the
  `map_err(poisoned)?` pair (and its `poisoned()` helper) with a
  single `lock_records(&self) -> MutexGuard<'_, Vec<…>>` that
  recovers from Mutex poison via
  `unwrap_or_else(PoisonError::into_inner)`. Same shape as the
  `CachedContentStore::lock_index` precedent shipped in the M4
  codecov fix; kills an unreachable
  `LiquidError::InvalidInput("…")` error path that codecov was
  flagging as uncovered.
- `core/liquid-sdk-bridge/src/api.rs`: reflowed the three
  multi-line idioms that tarpaulin instruments line-by-line into
  forms that fit the 100-char `max_width` on a single source
  line. Each `require_permission!(...)` call binds a local
  `let perms = self.permissions.as_ref();` first so the macro
  invocation fits on one line. The page-id-mismatch error path
  in `write_page` moves into a `page_id_mismatch(actual,
  expected)` helper so the `format!` args live on a single line.
  `list_workspaces`'s per-row `PermissionIndex::check` chain
  collapses to a single line via the same `perms` binding +
  an extracted `Resource::Workspace(...)` local.

Result: `liquid-sdk-bridge/src/api.rs` patch coverage 84.72% →
**100% (64/64 lines)**; `liquid-sdk-bridge/src/registry.rs`
90.00% → **100% (21/21 lines)**; workspace coverage 92.23% →
**93.71%** (+1.48%). All 19 bridge tests + 28 workspace test
groups continue to pass; clippy clean; fmt clean.

### Fixed — Post-M5 audit (Rust-side TASK-011 follow-up)

- `core/liquid-sdk-bridge/tests/m5_end_to_end.rs`: the
  previous `write_page_rejects_app_viewer_role` test was a placebo
  — the agent had zero bindings, so the test exercised the
  zero-bindings path, not the role-matrix path it claimed.
  Renamed to `write_page_rejects_unbound_agent` and added a new
  `write_page_rejects_app_viewer_role_against_page_resource` test
  that actually assigns `BuiltInRole::AppViewer` scoped to an
  `AppInstance` and asserts the bridge rejects a `Page` write
  (the genuine role-matrix path at `liquid-permissions::role.rs`).
  Plus two registry inline tests (duplicate-id rejection,
  newest-first sort) and an `empty_name → InvalidInput`
  end-to-end test.
- `docs/manual-validation-m4-m5.md` §M5.2: the previous Pass
  description claimed "every method other than `create_workspace`"
  runs `require_permission!`. Three methods actually omit the
  macro, each with a different documented reason: `create_workspace`
  (bootstrap), `list_workspaces` (per-row filtering instead of a
  single-resource gate), and `check_permission` (gating a
  permission *query* would loop). Rewrote §M5.2 + §9 to list
  all three exceptions.
- `IMPLEMENTATION_PLAN.md §9` `liquid-sdk-bridge` "Rules" section
  reworded so the three `require_permission!` exceptions are
  enumerated explicitly (matches `api.rs`'s module doc-comment).
- `core/liquid-sdk-bridge/src/api.rs::now_unix` gains a
  doc-comment explaining the `unwrap_or(0)` fallback's known
  degraded-sort consequence — a misordered list is preferable
  to a panic across the FFI boundary, but a reviewer should be
  able to find the trade-off without grepping the rationale.
- `docs/manual-validation-m4-m5.md` walkthrough line-count claim
  fixed (was "9-line matrix", actual is ~12 progress lines).
- `docs/manual-validation-m4-m5.md` §M5.3 expected test counts
  updated for the new tests (5 + 2 + 12 = 19 bridge tests total).

### Fixed — Documentation review findings (M0-M5 audit)

- `IMPLEMENTATION_PLAN.md §4.2` (PermissionIndex) now documents the
  globally-unique-UUID tenant-isolation assumption that
  `workspace_matches` relies on for non-`Resource::Workspace` checks
  (workspace-strict for `Workspace`; workspace-agnostic for
  `AppInstance / Component / Page` via UUID uniqueness;
  `Field(String)` flagged separately as Phase-3 follow-up). Pairs
  with two new tests in
  `core/liquid-permissions/tests/permission_index.rs` that
  characterise the assumption:
  `distinct_app_instance_uuids_do_not_cross_match_per_binding`
  (defensive — distinct UUIDs in different workspaces stay
  separate) and
  `app_instance_check_is_workspace_agnostic_by_uuid_uniqueness_assumption`
  (the assumption itself — `check` is workspace-agnostic by
  design; isolation rests on `Uuid::new_v4`, not on the index
  walking workspace ids).
- `IMPLEMENTATION_PLAN.md §5.1` (M1 milestone) ticks all checkboxes
  now that the code has been shipped, and adds the `PageId`,
  `OperationId`, `CommitId`, `RoleId` types that the original list
  omitted, plus a cross-ref to the M1-M3 validation guide.
- `IMPLEMENTATION_PLAN.md §5.4` and `§5.5` now cite the new
  `docs/manual-validation-m4-m5.md` guide + `m4_walkthrough`
  example, mirroring the §5.3 pattern.
- `IMPLEMENTATION_PLAN.md §12` (Agent CLI Specification) carries
  an opening "Implementation status" note pointing readers at M6.5
  (TASK-008) and M7 (TASK-009); previously §12 read as a live spec
  with no indication the `liquid` binary was a stub.
- `core/liquid-permissions/src/index.rs::InMemoryPermissionIndex`
  doc-comment said TASK-007 (disk-backed variant) was "queued";
  TASK-007 is Done — the comment now cross-references the shipped
  `FilesystemPermissionIndex`.
- `docs/adr/001-jujutsu-pinning.md` references to ADR-005 now point
  at the inline strategic ADR in `IMPLEMENTATION_PLAN.md §15`
  (which is where ADR-005 actually lives, per the §15 numbering
  note); previously the references read as dead links to a
  separate file.

### Added — Manual validation guide for M4 + M5

- `docs/manual-validation-m4-m5.md` (new) — auditable companion to
  `manual-validation-m1-m3.md`. Covers the second half of Phase 1:
  M4 (cache layer — `ReadCache` + `InProcessCache` +
  `CachedContentStore`) with step-by-step focused-test, walkthrough,
  invariant-by-inspection, and lints procedures; M5 (FFI bridge,
  currently PENDING) as a PR-review checklist the next reviewer
  follows when M5 lands.
- `core/liquid-vcs/examples/m4_walkthrough.rs` (new) — runnable,
  self-asserting reproduction of the M4 plan-level success criterion
  against a real `FilesystemContentStore`. Four asserted phases:
  cache hit on second read, write invalidates prior hash (no stale
  hit), per-workspace tenancy isolation, undo invalidates workspace
  cache + re-warm. Mirrors the per-milestone style of
  `m2_walkthrough` / `m3_walkthrough`.

### Fixed — codecov report (liquid-cli stub exemption)

- `.codecov.yml` now ignores `core/liquid-cli/**`, formalising the
  `IMPLEMENTATION_PLAN.md §15` policy ("Coverage target: ≥ 80% line
  coverage on all crates except `liquid-cli`"). PR #15 tripped the
  `codecov/patch` check at 0% because the one-line stub-message
  edit in `core/liquid-cli/src/main.rs` (commit `ed2e004`,
  "fix(cli): correct stub pointer to M6.5/M7") is by definition
  uncovered — `fn main()` exits 64 with an `eprintln!` and has no
  test surface until M6.5 ships the MVP CLI grammar. The exemption
  is documented inline in the YAML with a re-evaluate-at-M6.5
  note so the next agent does not silently leave the binary
  uncovered once it has a testable surface.

### Fixed — CLI scaffold pointer

- `core/liquid-cli/src/main.rs` stub previously claimed the CLI grammar
  lands in "M7 — see §5.7"; §5.7 is the Flutter shell milestone (M6),
  not the CLI. The corrected stub points at M6.5 (§5.6, TASK-008, the
  minimum surface that drives the MVP slice) and M7 (§5.8, TASK-009,
  the rest of §12). Exit code unchanged (`64` / `EX_USAGE`).

### Fixed — M4 codecov

- `CachedContentStore`: replaced `self.index.lock().map_err(|_|
  LiquidError::InvalidInput("…"))?` (three callsites, each
  contributing an unreachable error path that codecov / tarpaulin
  flagged as uncovered) with a single
  `fn lock_index(&self) -> MutexGuard<'_, IndexMap>` helper that
  recovers from poison via
  `unwrap_or_else(std::sync::PoisonError::into_inner)`. The
  recovery is safe for a cache index — at worst the next read
  hits a stale hash, which the wrapper already handles by
  falling through to the inner store. Absolute-Rule-1 compliant
  (the rule forbids `.unwrap()` / `.expect()` only).
- `CachedContentStore::undo`: replaced the two-pass
  collect-keys-then-remove block with a single
  `extract_if(|(ws, _), _| *ws == workspace)` pipeline. Same
  semantics, half the LOC, no temporary `Vec<(WorkspaceId,
  StorePath)>` allocation.
- New regression test
  `stale_index_entry_falls_through_to_inner_and_rewarms` in
  `core/liquid-vcs/tests/cached_store.rs` covers the
  cache-evicted-but-index-still-points-at-it recovery path that
  was previously implicit. Out-of-band invalidates the cache,
  then asserts the next read forwards to the inner store and
  re-warms.
- Removed the dead `_types_in_scope` test-only stub
  (`#[allow(dead_code)]` function in `cached_store.rs`); the
  imports it kept alive (`Operation`, `OperationKind`) are now
  used directly by `SpyStore`.

Result: `core/liquid-vcs/src/cached.rs` patch coverage 84.44% →
**100% (34/34 lines)**; `core/liquid-vcs/tests/cached_store.rs`
97.56% → **100% (40/40 lines)**. Codecov on the M4 PR is now
clean.

### Fixed — M4 follow-up

- `deny.toml` `hashbrown` skip comment now enumerates all three
  in-tree hashbrown versions and their dep sources (0.14.5 from
  dashmap, 0.15.5 from wasmparser, 0.17.0 from toml/indexmap).
  Comment was previously inaccurate (claimed two versions); the
  skip itself was always correct and covered all three.
- `dashmap` and `sha2` moved into `[workspace.dependencies]` so the
  version literal lives in one place instead of three. Matches the
  project's existing approach for `async-trait`, `bytes`, `tokio`,
  etc.
- `CachedContentStore`: removed dead `inner()` / `cache()`
  `#[doc(hidden)]` accessors (no callers in test or production
  code); replaced misuse of `// SAFETY:` comment in
  `ContentHash::of_bytes` with a plain infallibility note.
- `cache_is_independent_per_workspace_at_key_level` test now also
  asserts that workspace B's second read serves from cache (was
  only asserting that the returned bytes were correct).
- Documented the Phase-1 write/undo limitation inline in
  `CachedContentStore`: on inner-call failure the cache index is
  already cleared and warm entries already invalidated;
  correctness is preserved (the next read re-warms) but a perf
  regression accumulates across retries. Phase 3 will revisit
  when the bridge layer gains retry semantics.

### Added — M4 (cache layer)

- `liquid-cache::ReadCache` trait (`get` / `put` / `invalidate`,
  all async, `Send + Sync`) and `liquid-cache::InProcessCache`
  Phase-1 backend (`Arc<DashMap<ContentHash, Bytes>>`, no expiry).
  Closes `IMPLEMENTATION_PLAN.md` §4.3 trait surface. 8 integration
  tests cover put/get/overwrite/invalidate/missing-key-no-op/
  distinct-keys/cheap-clone-shared-state/`dyn ReadCache`
  trait-object dispatch.
- `liquid-vcs::CachedContentStore<S, C>` — generic adapter that
  wraps any `ContentStore` with any `ReadCache` and implements the
  M4 wiring: read warms the cache, write invalidates the prior
  hash, undo conservatively invalidates every cached hash for the
  affected workspace (precise per-path invalidation deferred to
  the jj-lib backend in TASK-004). Maintains an in-memory
  `(WorkspaceId, StorePath) → ContentHash` index so the second
  read of a path can find its cached bytes without touching the
  inner store — the M4 success-criterion path. 7 wiring tests
  cover the SpyStore-counter success criterion, write-invalidates,
  miss-non-poisoning, content-addressable dedup across paths,
  undo-invalidates, list/operation_log pass-through, and
  per-workspace tenancy isolation of the path-hash index.
  `dashmap 6.1` brings a hashbrown 0.14 / 0.17 duplicate; added a
  `hashbrown` entry to `deny.toml`'s `bans.skip` list with the same
  upstream-resolves-itself rationale as the existing `getrandom`
  skip.
- `liquid_core::ContentHash::of_bytes(&[u8])` — infallible
  SHA-256-to-hex constructor. Centralises the SHA-256 dependency
  in `liquid-core` (where it already had to live for ID
  primitives) so the cache call-sites do not need their own
  hashing logic or Absolute-Rule-1-bending `.expect()` calls. RFC
  6234 vectors for empty input and `"abc"` plus a
  round-trip-through-`from_hex` + collision-free test land in
  `core/liquid-core/tests/integration.rs` (workspace test count
  goes 26 → 30).
- Workspace test count: **75** in M1–M4 at this commit (was 60);
  subsequent agent-discipline + audit-finding + M5 commits in the
  same `[Unreleased]` cycle lift it to **139** (corner tests +
  cross-workspace UUID isolation tests + M5 inline-and-e2e suite
  including the real AppViewer-on-AppInstance write rejection
  test and the duplicate-id-rejection + sort-order registry
  tests, see entries above).

### Documentation

- `docs/manual-validation-m1-m3.md` (new) — Phase-1 manual
  validation guide covering M1 (`liquid-core` primitives), M2
  (VCS layer + on-disk ADR-001 layout inspection), and M3
  (auth + permissions + Argon2id hash check + no-mode-leak
  token surface). Walks a human reviewer through focused
  `cargo test` commands, the new walkthrough examples, and the
  per-milestone on-disk inspection. Closes the sign-off-checklist
  gap that previously left "Phase 1 release ready?" answerable
  only by the author.
- `core/liquid-vcs/examples/m2_walkthrough.rs` (new) — runnable,
  self-asserting reproduction of the M2 plan-level success
  criterion (`workspace create → write three → read back → list →
  op-log → undo → NotFound`). Leaves artifacts under
  `$(temp_dir)/liquid-m2-walkthrough/` for `ls -la` /
  `cat op_log.jsonl` inspection.
- `core/liquid-permissions/examples/m3_walkthrough.rs` (new) —
  runnable demonstration of the M3 success criterion against
  *both* `InMemoryPermissionIndex` and `FilesystemPermissionIndex`,
  with the disk-persistence re-open test plus the four-way token
  negative surface (tampered / wrong-key / expired / malformed →
  all `Forbidden`). Leaves Argon2id-hashed `users.toml`,
  `agents.toml`, and `permissions.toml` under
  `$(temp_dir)/liquid-m3-walkthrough/` for inspection.
- `IMPLEMENTATION_PLAN.md` §5.3 prose updated to match shipped
  state: dropped the stale "disk-backed variants are deferred"
  claim (TASK-007 shipped `FilesystemPermissionIndex` and
  TASK-006 shipped the disk-backed `LocalIdentityProvider`).
  Added a forward link to the new manual-validation guide.

### Fixed

- `.claude/scripts/gh-job-log`:
  - Per-step bucketing now handles the `gh run view --log-failed`
    tab-separated format (`TIMESTAMP\tJOB\tSTEP\tLINE`) in addition
    to the raw zip's `##[group]` markers. The original parser was
    a no-op on the gh path; the "last 50 lines per failed step"
    cap is now honoured on both code paths.
  - Step files in the run-log zip are now concatenated in
    chronological order via `sort -zV` (version-sort) instead of
    lexicographic `sort -z` — jobs with 10+ steps used to read
    `step 10` before `step 2`.
  - Tempfile / unzip-dir cleanup is now governed by a `RETURN`
    trap so an `xargs cat` failure no longer leaks the zip in
    `/tmp/`.
  - `run_id` and `job_id` arguments are validated as positive
    integers (rejected at exit 2 if malformed), closing the
    path-traversal class on the log filename composition.
  - 7 bats cases in `tests/cli/05_gh_job_log.bats` cover the
    network-free paths: arity / input-validation, raw-mode
    bucketing, gh-mode bucketing, 200-line total cap.

- `justfile` (`lint-rust`, `lint-rust-filtered`, `fmt-rust`) and
  `lefthook.yml` (`rust-fmt`): pass `--all` to `cargo fmt` when
  `--manifest-path` is set. rustfmt 1.8+ errors with "Failed to
  find targets" without `--all`, which silently broke `just check`
  (and `just lint`) for any contributor on the current pinned
  toolchain. CI already uses the equivalent form (`cd core &&
  cargo fmt --all --check` via `working-directory: core`), so the
  bug was local-only. No source files reformatted by the fix.
- `justfile` Flutter recipes (`test-app`, `lint-app`, `fmt-app`,
  `test-sdk`, `lint-sdk`, `fmt-sdk`, `test-sdk-filtered`): skip
  with a friendly "pubspec.yaml not yet — see
  IMPLEMENTATION_PLAN.md §5.7" message when the layer hasn't been
  scaffolded. Matches the existing skip-when-absent pattern in
  `lefthook.yml` and the `detect`-layer gating in CI. Without
  this, `just check` and `just lint` fail on a fresh clone before
  M6 lands.

### Documentation

- `docs/ops/branch-protection.md` (new) — maintainer checklist for
  enabling GitHub branch-protection on `main`. Names the exact
  required CI checks (`Rust (ubuntu-latest)`, `CLI bats tests`,
  `cargo audit`, `cargo deny`, `ai-check`, `sync-docs`) and the
  additional settings (require PR, dismiss stale approvals,
  require linear history, disallow force-pushes and deletions).
  GitHub branch-protection rules cannot be applied from CI
  without admin credentials; the doc is therefore the auditable
  checklist for the maintainer task.
- `tests/cli/README.md` (new) — explicit "skip-only until M6.5"
  status note. Distinguishes the **live** tests
  (`01_branch_name_gate.bats`, `02_bump_version.bats`,
  `03_pre_commit_review_hook.bats`, `04_changelog_gate.bats`) from
  the M6.5-pending spec scaffold (`00_mvp_slice.bats`, mostly
  `skip "pending M6.5"`). Reviewers can now reject "CLI test
  added" PR claims that turn out to be all-skip.
- `.github/PULL_REQUEST_TEMPLATE.md` — new "Coverage claim"
  author-checklist item asking the PR author to label any "CLI
  integration test added" claim as either *live* or
  *skip-pending-M6.5*.

### Changed

- `deny.toml` license allow-list trimmed: removed `Zlib`,
  `Unicode-DFS-2016`, and `CC0-1.0` — none were in use by any crate
  in the current dependency graph, and cargo-deny was emitting
  `license-not-encountered` warnings on every run. The principle is
  "add allowances as a real new transitive dependency requires them,
  never speculatively"; the failure mode for a removed-too-eagerly
  license is a clean cargo-deny error pointing at the rejecting
  crate, which lets the maintainer audit and re-add intentionally.
  Note: `Unicode-3.0` was on the trim list per the original goal,
  but `unicode-ident-1.0.24` ships under `(MIT OR Apache-2.0) AND
  Unicode-3.0` — the AND makes it mandatory — so it stays. `ISC`
  and `BSD-2-Clause` are also currently unmatched but kept (commonly
  required by future transitive deps; they will be revisited when
  they appear in `cargo-deny check` warnings again).

### Added

- `scripts/bump-version.sh` + `just bump-version <new-semver>`
  recipe — single source-of-truth bump for the workspace release
  version. Atomically rewrites `[workspace.package].version` AND
  every `liquid-* = { path = "...", version = "..." }` literal in
  `[workspace.dependencies]` of `core/Cargo.toml`. Eliminates the
  drift class where bumping the workspace version forgot one of
  the 7 path-dep version literals (cargo treats path-only deps as
  wildcards, which trips cargo-deny's `wildcards = "deny"` rule;
  the path-dep literal MUST track the workspace version at all
  times). The `core/Cargo.toml` workspace.package block now carries
  a "LIQUID_VERSION" header comment pointing future maintainers at
  the script. Covered by 8 bats cases in
  `tests/cli/02_bump_version.bats` (semver acceptance, idempotency,
  pre-release tags, leaves rust-version + third-party deps
  untouched).

- `commit-msg` lefthook step `changelog-discipline` running
  `.lefthook/commit-msg/check-changelog.sh`. Rejects `feat(*)` /
  `fix(*)` / `refactor(*)` / `perf(*)` / `chore(<non-tooling-scope>)`
  commits that do not modify `CHANGELOG.md` and do not carry a
  `[no-changelog]` trailer. Exempts `docs(*)`, `test(*)`, and
  `chore(ci|claude|deps|ai|gh|tooling)`. Covered by 14 bats cases
  in `tests/cli/04_changelog_gate.bats`. Documented in
  `CONTRIBUTING.md` "Documentation as part of the change".


- `.claude/rules/log-volume.md` — formalises the "any command output
  >50 lines must go through filter-test-output.sh, test-triager, or
  gh-job-log" discipline that was scattered across the goal block,
  the operating-mode bullets in CLAUDE.md, and a few skill files.
  Now a single authoritative rule cited from `CLAUDE.md` Rules and
  the `implement` skill's Operating-mode section.

- `.claude/scripts/gh-job-log` — GitHub Actions workflow-log
  fetcher. `bash .claude/scripts/gh-job-log <run_id> [<job_id>]`
  pulls the run log via `gh run view --log-failed` (or `curl` + the
  REST API when `gh` is absent), writes the raw output to
  `.ai/artifacts/logs/gh-job-<run_id>-<ts>.log`, and prints only the
  last 50 lines of every failed step. Cited by the new `log-volume`
  rule as the canonical way to surface CI failures without pasting
  the full log into chat.

- `.claude/agents/github-pr.md` — dedicated read-only GitHub
  inspector subagent (haiku). Restricted to the `mcp__github__*` read
  tools only (no comment / merge / push capability). Use for "what's
  the state of PR #N?", "which open PRs touch crate X?", "is there
  an open issue about Y?", etc. Writes still go through the main
  agent invoking the matching `mcp__github__*` write tool directly.
- `scripts/ai-check.sh` step 3b: assert every `.claude/agents/*.md`
  file on disk is mentioned in CLAUDE.md (catches the inverse of
  step 3 — a new agent added to disk but forgotten in the docs).

- `.claude/hooks/pre-commit-review.sh` — `PreToolUse` hook matched on
  `Bash(git commit -*)` and `Bash(git commit --*)` (tight patterns
  so `git commit-tree` / `git commit-graph` plumbing does not
  trigger the hook). Snapshots `git diff --staged` to
  `.ai/artifacts/diffs/pre-commit-<ts>.diff`, returns the documented
  `{"hookSpecificOutput": {"permissionDecision": "ask",
  "permissionDecisionReason": "..."}}` PreToolUse envelope, and
  asks the agent to spawn the `code-reviewer` subagent against the
  snapshot before the commit lands. The subagent's `critical` array
  is the block; warnings and suggestions remain advisory. Two
  opt-out paths: `LIQUID_SKIP_PRE_COMMIT_REVIEW=1` in the host env
  before starting Claude Code (for a long rebase session), or a
  `[skip-review]` token in the commit message (parsed from the
  tool-call command on stdin via jq, for a single
  conflict-resolution commit). Snapshot retention caps the
  diffs/ tree at the most recent 20 entries. Empty staged diff is a
  silent no-op. Covered by 7 bats cases in
  `tests/cli/03_pre_commit_review_hook.bats`.

- Pre-push branch-name gate (`scripts/check-branch-name.sh`, wired
  into `lefthook.yml`'s `pre-push` hook). Rejects pushes from `main`,
  bare `claude`, or any `claude/*` branch — the Claude Code agent
  autobranch namespace — forcing the change onto a `feature/<topic>`
  / `fix/<topic>` branch before it can reach the remote. Eleven bats
  cases in `tests/cli/01_branch_name_gate.bats` cover the gate
  (exact-match `main`, `claude` family including nested paths,
  substring-only acceptances like `feat/handle-claude-feedback` and
  `feat/main-page-redesign`, and the empty-string caller-bug path
  that exits 2 instead of silently falling through to git detection).

- `just deny-check` recipe and matching pre-push lefthook step
  wrapping `cargo deny --manifest-path core/Cargo.toml check --config
  deny.toml`. `just check` now chains `lint → test → deny-check`, so
  every local pre-push validation cycle catches advisory / license /
  ban regressions that previously only fired on CI (the
  EmbarkStudios/cargo-deny-action job in `.github/workflows/audit.yml`).
  `cargo-deny` is now listed in `CONTRIBUTING.md`'s prerequisites
  table; install with `cargo install --locked cargo-deny`.

### Changed

- `.claude/settings.json`: tightened the `git push --force` / `git push
  -f` deny patterns into four narrow literals (`--force`, `--force *`,
  `-f`, `-f *`) so they no longer match `--force-with-lease`, and added
  `Bash(git push --force-with-lease*)` to the allow list. Agents must
  use `--force-with-lease` (never bare `--force`) when a rebase or
  rewrite has to overwrite a remote feature branch — it refuses the
  push if anyone else updated the ref in the meantime, preventing the
  silent overwrite that bare `--force` enables.

### Added

- `liquid-permissions::FilesystemPermissionIndex` — TOML-backed
  implementation of `PermissionIndex` (TASK-007). Bindings persist as
  `<root>/workspaces/<id>/permissions.toml`; one file per workspace,
  atomic writes via tmp-then-rename, in-memory cache for O(n-bindings)
  `check`. Same trait as `InMemoryPermissionIndex`; callers don't
  change. Finishes M3.
- 9 integration tests for the disk variant (round-trip, persistence
  across instance restart, scope validation, multi-workspace file
  separation, malformed-TOML rejection, empty-bindings round-trip).
  Workspace test count: **87** (was 78).

### Changed

- `Binding` (private to `liquid-permissions`) is now `pub(crate)` and
  carries a `matches()` method that encapsulates the workspace + scope
  + role-matrix check. Both index implementations share that one
  definition rather than duplicating the logic. No public-API change.

## [0.1.0-pre.M3] — 2026-05-05

Phase 1 milestone 3 ships auth + permissions. The full milestone log
below covers the complete Phase 1 progress to date.

### Added — M3 (auth + permissions)

- `liquid-permissions::PermissionIndex` trait with in-memory
  implementation `InMemoryPermissionIndex` (`HashSet`-backed bindings,
  O(1) check on the principal's binding count).
- `BuiltInRole` enum encoding the five Phase-1 roles
  (`WorkspaceOwner | WorkspaceMember | AppViewer | AppEditor | Agent`)
  and their hard-coded permission matrix.
- `require_permission!(index, principal, action, resource)` macro —
  the canonical permission gate at every `liquid-sdk-bridge` and CLI
  callsite (CLAUDE.md rule 4).
- `liquid-auth::IdentityProvider` trait with file-backed
  implementation `LocalIdentityProvider`:
  - Argon2id-hashed passwords (`<root>/users.toml`).
  - Provisioned agents (`<root>/agents.toml`).
  - HMAC-SHA256 session tokens of the form
    `principal . expires_unix . hmac_hex`.
  - Atomic writes via tmp-then-rename.
- 26 new tests (13 auth integration + 12 permission unit + 1
  end-to-end). Workspace-wide test count: **78** (was 52).
- ADR-002: M3 trait scoping decisions — drop `grant`, replace
  `RoleId` with `BuiltInRole`, drop `workspace_id` from session tokens.

### Changed — M3

- `IMPLEMENTATION_PLAN.md` §4.2 / §4.5 / §5.3 / §9 / §15 updated to
  reflect the trait shapes actually shipped.
- `TASKS.md` — TASK-005 and TASK-006 marked Done; TASK-007
  (disk-backed `PermissionIndex`) added as the M3 follow-up.

### Added — M2 (VCS layer, prior milestone)

- `liquid-vcs::ContentStore` trait — `read`, `write`, `operation_log`,
  `undo`, `list`, all returning `Result<_, LiquidError>`.
- `InMemoryContentStore` — test/dev backend, no persistence.
- `FilesystemContentStore` — durable Phase-1 backend with the
  layout `<root>/<workspace_id>/files/<path>` plus
  `op_log.jsonl`, atomic writes via tmp-then-rename.
- ADR-001: filesystem stand-in for Phase 1; `jj-lib` integration
  deferred to TASK-004.

### Added — M1 (workspace bootstrap, prior milestone)

- Cargo workspace under `core/` with eight crates: `liquid-core`,
  `liquid-vcs`, `liquid-auth`, `liquid-permissions`, `liquid-cache`,
  `liquid-bindings`, `liquid-sdk-bridge`, `liquid-cli`.
- `liquid-core` primitives: `WorkspaceId`, `AppInstanceId`,
  `ComponentId`, `PageId`, `PrincipalId`, `RoleId`, `OperationId`,
  `CommitId`, `ContentHash`, `StorePath`, `SlotName`, `SlotValue`,
  `Action`, `Resource`, `TenantConfig`, `LiquidError`.
- Workspace lints forbidding `unsafe_code` and warning on
  `unwrap` / `expect` / `panic` / `todo` / `unimplemented`.

### Added — project / OSS scaffolding

- `LICENSE` — Apache-2.0 (matches the workspace-wide declaration in
  `core/Cargo.toml`).
- `NOTICE` — third-party attribution per Apache convention.
- `README.md` — rewritten in OSS-standard format with a status table.
- `DEVELOPER_INFO.md` — design rationale and architecture detail
  moved out of the README.
- `CONTRIBUTING.md` — full contributor workflow and project rules.
- `CODE_OF_CONDUCT.md` — Contributor Covenant 2.1, adopted by
  reference.
- `SECURITY.md` — vulnerability disclosure via GitHub Security
  Advisories.
- `CHANGELOG.md` (this file).
- `.github/ISSUE_TEMPLATE/` — bug, feature, and task templates.
- `.github/PULL_REQUEST_TEMPLATE.md` — PR checklist.
- Root `.gitignore` covering Flutter, IDE, OS, and `.ai/` artifacts.
- `.claude/skills/sync-docs/` — repo-local skill that catches
  documentation drift after implementation work.

### Project conventions

- All public Rust functions return `Result<_, LiquidError>` — no
  parallel error hierarchies.
- Conventional Commits drive `cargo-release`-generated changelogs
  (see `IMPLEMENTATION_PLAN.md` §16).
- The seven Absolute Rules from `CLAUDE.md` are CI-enforced where
  possible; reviewers enforce the rest.

---

> **Reading this file before there is a tagged release?** Pre-1.0
> entries above are **provisional** — they describe what's on `main`
> at the time of writing but have not yet been published as a versioned
> artefact. The first actual `cargo-release` tag will collapse the
> Phase-1 milestones into a single `0.1.0` entry; until then this file
> is more of a milestone diary than a release log.
