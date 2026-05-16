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

### Added

- `.claude/hooks/pre-commit-review.sh` — `PreToolUse` hook matched on
  `Bash(git commit*)`. Snapshots `git diff --staged` to
  `.ai/artifacts/diffs/pre-commit-<ts>.diff`, returns
  `decision: "ask"`, and asks the agent to spawn the `code-reviewer`
  subagent against the snapshot before the commit lands. The
  subagent's `critical` array is the block; warnings and suggestions
  remain advisory. Opt-out via `LIQUID_SKIP_PRE_COMMIT_REVIEW=1` for
  rebase / conflict-resolution commits. Empty staged diff is a silent
  no-op.

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
