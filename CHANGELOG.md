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
  subsequent agent-discipline + audit-finding commits in the same
  `[Unreleased]` cycle lift it to **121** (corner tests +
  cross-workspace UUID isolation tests, see entries above).

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
