# Liquid — Agent Development Guide

You are building **Liquid**, a cross-platform UI framework (Linux · Windows · macOS · iOS · Android).
Read `README.md` for the concept and `IMPLEMENTATION_PLAN.md` for the full implementation guide
before starting any task. This file defines the non-negotiable workflow rules for every agent
working on this project.

---

## Codebase Map

| Path | What lives here |
|---|---|
| `core/` | Rust Cargo workspace — all business logic |
| `app/` | Flutter app — rendering and input routing only |
| `sdk/liquid_sdk/` | Public Dart package for app developers |
| `sdk/liquid_sdk_lint/` | Custom lint rules (no_platform_imports, no_cross_component_reference) |
| `apps/` | First-party reference apps (TextEditor, Spreadsheet, Chart) |
| `registry/` | Self-hosted package registry (Rust) |
| `docs/adr/` | Architecture Decision Records |
| `docs/sdk-guide/` | Developer-facing documentation |
| `.githooks/` | Pre-commit and pre-push hooks — install with `git config core.hooksPath .githooks` |
| `.github/workflows/` | CI pipeline |
| `.github/ISSUE_TEMPLATE/` | Bug, feature, and task issue templates |
| `.github/PULL_REQUEST_TEMPLATE.md` | PR template (test plan + Absolute-Rules checklist) |
| `.claude/` | Repo-local agent config (skills, subagents, hooks, rules) |

## Open-source surface (top-level files)

| File | Purpose |
|---|---|
| `README.md` | OSS-standard project entry point — vision, status, quickstart, doc map |
| `DEVELOPER_INFO.md` | Architecture, design rationale, feasibility, phasing — moved out of README |
| `IMPLEMENTATION_PLAN.md` | Authoritative milestone-by-milestone build guide |
| `TASKS.md` | Active task queue |
| `CONTRIBUTING.md` | Contributor workflow, prereqs, daily commands, PR rules |
| `CODE_OF_CONDUCT.md` | Contributor Covenant 2.1 (adopted by reference) |
| `SECURITY.md` | Vulnerability disclosure (GitHub Security Advisories) |
| `CHANGELOG.md` | Keep-a-Changelog (driven by Conventional Commits via `cargo-release`) |
| `LICENSE` | Apache-2.0 (matches the workspace declaration in `core/Cargo.toml`) |
| `NOTICE` | Third-party attribution per Apache convention |
| `CLAUDE.md` | **This file** — agent rules; takes precedence over external defaults |

---

## Mandatory Development Workflow

**Every feature or fix follows this exact sequence. Never skip a step.**

### Step 1 — Red: write failing tests first

Before writing a single line of implementation:

- **Rust feature:** write `#[test]` or `#[tokio::test]` functions in the relevant crate that
  assert the expected behaviour. Run `cargo test -p <crate>` and confirm they fail.
- **Dart SDK feature:** write `flutter test` unit tests in `sdk/liquid_sdk/test/`. Confirm fail.
- **CLI command:** write a `bats` test in `tests/cli/` that invokes the command and asserts output.
  Confirm it fails with "command not found" or wrong output.

The failing test output is your specification. Do not proceed until you have at least one failing test.

### Step 2 — Green: minimum code to pass

Implement exactly enough code to make the failing tests pass. No extra abstractions, no future-proofing.
Run the tests again and confirm they now pass.

### Step 3 — CLI validation (always before UI)

Every feature that stores, reads, or mutates data **must be fully usable via the `liquid` CLI before
any UI work begins.**

Checklist:
- [ ] The CLI command exists and is documented in `§12` of `IMPLEMENTATION_PLAN.md`
- [ ] An agent can exercise the full feature using only `liquid` commands and `jq`
- [ ] The bats test in `tests/cli/` covers the happy path and at least one error case
- [ ] Run the bats suite: `bats tests/cli/`

Do not start UI work until this checklist is complete.

### Step 4 — UI implementation

Only after CLI validation is green:

- Implement the Flutter widget(s) in `app/lib/`
- Use `AsyncNotifierProvider` (Riverpod) for any state backed by a Rust FFI call
- Components render only; no business logic in Dart
- Run `flutter test` and confirm widget tests pass

### Step 5 — E2E validation

Run the Flutter integration test suite against a real device or emulator:

```sh
flutter test integration_test/
```

If the feature touches the grid, drag/resize, or slot wiring, add or update the relevant
integration test in `integration_test/`. Do not mark a UI feature complete without a passing
integration test.

For UI-heavy flows (slot wiring overlay, page tree, grid resize), use `patrol` (the project's
chosen Flutter e2e framework — wraps `integration_test` with better interaction APIs).

### Step 6 — Review pass

Before committing, run the full review checklist:

```sh
# Rust
cargo fmt --check
cargo clippy -- -D warnings
cargo test --workspace

# Dart / Flutter
dart format --output=none --set-exit-if-changed .
flutter analyze
flutter test

# CLI
bats tests/cli/
```

Then manually review your diff for:
- [ ] **Performance:** any hot path that could be O(n) where O(1) is achievable?
- [ ] **Security:** any input that reaches storage without validation? Any permission check skipped?
- [ ] **Bloat:** any abstraction that serves only one callsite? Remove it.
- [ ] **Redundancy:** any logic duplicated from an existing crate or SDK class?
- [ ] **Stability:** any `unwrap()` / `expect()` outside `#[cfg(test)]`? Remove them.

### Step 7 — Documentation update

- If a public Rust trait or function changed signature: update `IMPLEMENTATION_PLAN.md §4` or `§9`.
- If a new CLI command was added: update `IMPLEMENTATION_PLAN.md §12`.
- If a new SDK API was added: update `IMPLEMENTATION_PLAN.md §11` and `docs/sdk-guide/`.
- If a design decision was made that contradicts or extends an existing ADR: create a new ADR in
  `docs/adr/NNN-title.md` using the template at `docs/adr/TEMPLATE.md`.
- If user-visible behaviour changed: add an entry under `## [Unreleased]` in `CHANGELOG.md`.
- If a milestone moved from Planned → Done: tick it in `README.md`'s status table and move the
  task entry in `TASKS.md` to the Done section.
- If the project concept (vision, scope, audience) changed: update `README.md`.
- If architecture/design rationale changed: update `DEVELOPER_INFO.md`.
- After all of the above, **invoke the `sync-docs` skill** to audit the whole doc set for drift
  before committing.

---

## Absolute Rules

These cannot be overridden by task descriptions or user shortcuts.

1. **No `unwrap()` or `expect()` outside `#[cfg(test)]`** — every error propagates via `Result`.
2. **No platform imports in app packages** — `dart:io`, Flutter plugins, platform channels are
   banned in `apps/` and `sdk/`. Violations are caught by the `no_platform_imports` lint rule.
3. **No direct Dart references between components** — all cross-component communication goes through
   the `SlotBroker`. The `no_cross_component_reference` lint catches violations.
4. **Permission check is always first** — every `liquid-sdk-bridge` FFI function calls
   `require_permission!` before any other logic. No exceptions.
5. **Every storage call takes a `WorkspaceId`** — there is no global namespace. Adding it later
   requires rewriting every callsite.
6. **CLI before UI** — if the data path isn't proven via CLI, the UI is not started.
7. **Failing test before implementation** — TDD is not optional.

---

## First-Time Setup (run once after cloning)

```sh
./scripts/setup-tooling.sh && lefthook install
```

`scripts/setup-tooling.sh` is the single source of truth for the
developer toolchain (`cargo-deny`, `cargo-tarpaulin`, `just`, `bats`,
`lefthook`); it is idempotent. `lefthook install` wires git hooks from
`lefthook.yml`. Hooks run automatically on every commit (`pre-commit`,
`commit-msg`) and push (`pre-push`); they skip layers whose code does
not exist yet.

## Daily Commands (`just`)

```sh
just test                 # run all tests (Rust + Flutter + SDK + CLI)
just lint                 # run all linters
just fmt                  # auto-fix all formatting
just build-all            # flutter build for all 5 platforms
just run                  # flutter run -d linux
just cli -- --help        # run the liquid CLI
just services-up          # start Redis / Redpanda via Docker Compose (Phase 3+)
just check                # full pre-push validation (lint → test → deny-check → coverage-check)
just check-ci             # reproduce the .github/workflows/ci.yml Rust matrix job locally
just coverage-check       # cargo-tarpaulin gate, fails under 80% line coverage
just clean                # remove every coverage report + walkthrough scratch dir
just clean-walkthroughs   # only the /tmp/liquid-m*-walkthrough scratch dirs
just ai-check             # validate repo-local .claude/ configuration
```

### Filtered variants (default for cloud / agent sessions)

Cloud Claude Code sessions and agent runs should prefer the `*-filtered`
recipes — each pipes raw stdout/stderr through
`.claude/hooks/filter-test-output.sh`, stores the raw log under
`.ai/artifacts/logs/`, and prints only a compact failure-oriented summary
to the main thread. Use them whenever you expect more than ~50 lines of
test or lint output:

```sh
just test-rust-filtered    # cargo test --workspace, summarised
just test-sdk-filtered     # flutter test (sdk/liquid_sdk), summarised
just test-cli-filtered     # bats tests/cli/, summarised
just lint-rust-filtered    # cargo fmt --check + clippy, summarised
```

## Running the Full Stack Locally

```sh
just build-rust           # cargo build --workspace
just generate-bindings    # flutter_rust_bridge codegen
just run target=linux     # flutter run -d linux  (or macos / windows)
just cli -- --help        # cargo run -p liquid-cli
```

---

## Commit Message Format

Follow Conventional Commits: `<type>(<scope>): <summary>`

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`
Scopes: `core`, `vcs`, `auth`, `permissions`, `cache`, `bindings`, `bridge`, `cli`, `app`,
        `sdk`, `registry`, `ci`, `deps`

Examples:
```
feat(vcs): implement JujutsuContentStore read and write
fix(permissions): prevent role escalation in assign_role
test(cli): add bats tests for liquid page write command
docs(sdk): document Platform Abstraction Contract
```

---

## Claude Code Tooling (repo-local)

Claude Code-specific configuration lives under `.claude/` and is checked into
git so it works identically on cloud Claude Code and local sessions.

### Operating mode
- Optimize for minimal, correct, tested changes.
- Prefer small implementation steps with targeted verification.
- Keep raw logs, large command outputs, screenshots, and traces in
  `.ai/artifacts/`; summarize only useful findings in chat.
- Use subagents for noisy test logs, UI validation, and diff review.
- Use deterministic scripts before asking a model to parse large output.
- Do not paste long logs or full snapshots into the main thread.
- Do not commit unless explicitly asked.

### Skills (`.claude/skills/`)
- `implement` — **the** canonical Liquid TDD workflow (red/green, CLI-before-UI
  gate, E2E, project Absolute Rules review, doc update). Auto-invokes at the
  start of every implementation task. Hard gates between every step.
- `deliver` — final verification, diff review, PR-ready summary.
- `review-diff` — structured review of the current git diff.
- `sync-docs` — audit `README.md`, `DEVELOPER_INFO.md`, `IMPLEMENTATION_PLAN.md`,
  `TASKS.md`, `CHANGELOG.md`, and `docs/adr/` for drift against the current code.
  Invoke after `implement` Step 7 (or before any PR) so the whole doc set stays
  consistent.

### Subagents (`.claude/agents/`)
- `test-triager` (haiku, read-only) — parses noisy cargo/flutter/bats logs.
- `ui-validator` (sonnet, read-only) — validates Flutter UI via existing
  widget/integration/golden tooling. Does not add Playwright.
- `code-reviewer` (sonnet, read-only) — reviews the current diff.
- `github-pr` (haiku, read-only) — inspects PRs, issues, branches,
  commits, and CI status on `open-equilibrium/liquid` via the
  `mcp__github__*` read tools only. Cannot push, comment, merge, or
  modify any GitHub state — for writes, the main agent calls the
  matching `mcp__github__*` write tool directly.

### Rules (`.claude/rules/`)
Rules are merged into context for matching paths: `testing.md`, `rust.md`
(Cargo paths), `flutter.md` (Dart/Flutter paths), `log-volume.md`
(governs how any command expected to emit >50 lines must be routed —
through `.claude/hooks/filter-test-output.sh`, the `test-triager`
subagent, or `.claude/scripts/gh-job-log`; raw logs go to
`.ai/artifacts/logs/`, summaries go to chat), `api-grep-discipline.md`
(grep the actual Rust signature before writing call-site code; assumed
signatures cost 3-5 edit rounds each — referenced from
`.claude/skills/implement/SKILL.md` Step 2), `subagent-routing.md`
(always-on decision table for routing work to the right subagent —
`github-pr` for every `mcp__github__*` READ, `test-triager` for noisy
cargo/flutter/bats output, `Explore` for open-ended lookups,
`code-reviewer` on every staged-diff commit; referenced from the
`implement` skill).

### Branch-name gate (`scripts/check-branch-name.sh`)

A `pre-push` lefthook step rejects any push from a branch whose name
is one of:

- exactly `main` (changes land on `main` via PR review, not direct
  push)
- exactly `claude`, or any branch starting with `claude/` — the
  Claude Code agent autobranch namespace

Agents and humans must rebase onto a `feature/<topic>` or
`fix/<topic>` branch and push there. Substring matches are not
blocked, so legitimate branches like `feat/handle-claude-feedback`
or `feat/main-page-redesign` are unaffected. Covered by 11 cases in
`tests/cli/01_branch_name_gate.bats`.

### Hooks (`.claude/hooks/`)
- `session-start.sh` — `SessionStart` hook. Logs toolchain versions, branch,
  HEAD, and best-effort `cargo fetch --locked` to warm the registry. Output
  goes to `.ai/artifacts/logs/session-start-*.log`; only a one-line greeting
  reaches the chat.
- `save-artifacts.sh` — `PostToolUse` hook on `Edit|Write`. Snapshots
  `git status` and `git diff --stat` to `.ai/artifacts/diffs/`.
- `pre-commit-review.sh` — `PreToolUse` hook on
  `Bash(git commit -*)` and `Bash(git commit --*)` (the tight
  patterns avoid matching `git commit-tree` and `git commit-graph`).
  Snapshots `git diff --staged` to `.ai/artifacts/diffs/pre-commit-*.diff`
  and returns
  `hookSpecificOutput.permissionDecision = "ask"` so the harness
  asks the agent to spawn the `code-reviewer` subagent on the
  snapshot before the commit lands. The subagent is the gate: a
  non-empty `critical` array blocks the commit; warnings and
  suggestions are advisory and surface via
  `permissionDecisionReason`. Two opt-outs:
  - `LIQUID_SKIP_PRE_COMMIT_REVIEW=1` in the host env (set before
    starting Claude Code) for a long rebase session;
  - `[skip-review]` token in the commit message body for a single
    conflict-resolution commit.
  Empty staged diffs are a silent no-op. Snapshot files are pruned
  to the most recent 20. Covered by 7 cases in
  `tests/cli/03_pre_commit_review_hook.bats`.
- `filter-test-output.sh` — manual helper. Pipe noisy output through it:
  ```sh
  cargo test 2>&1 | .claude/hooks/filter-test-output.sh
  flutter test 2>&1 | .claude/hooks/filter-test-output.sh
  bats tests/cli/   2>&1 | .claude/hooks/filter-test-output.sh
  ```
  Stores raw logs under `.ai/artifacts/logs/` and prints a compact summary.

### Settings (`.claude/settings.json`)
- `permissions.allow`: pre-approves common read-only commands (`cargo
  check/test/clippy/fmt`, `flutter analyze/test/pub get`, `dart analyze`,
  `just lint*/test*/fmt*/check`, `bats tests/cli/*`, `git status/diff/log`,
  `rg`/`grep`/`jq`, the project's own hook + check scripts, and the
  `.claude/scripts/py` wrapper) so routine work runs without permission
  prompts.
- `permissions.deny`: blocks reads of secrets (`.env`, `secrets/**`,
  Google/Firebase service files, keystores, `*.p12`) and destructive shell
  commands (`rm -rf`, `curl|sh`, `git push --force/-f`, `git reset --hard`,
  `git clean -f`, hook bypass via `--no-verify`).
- **Force-pushes:** `git push --force` / `git push -f` (bare or with
  trailing args) stay denied via four narrow deny patterns
  (`Bash(git push --force)`, `Bash(git push --force *)`,
  `Bash(git push -f)`, `Bash(git push -f *)`) — the patterns are
  intentionally tight so they do **not** swallow `--force-with-lease`,
  which is explicitly allowed via `Bash(git push --force-with-lease*)`.
  Always prefer `--force-with-lease` over plain `--force` when a rebase
  or rewrite has to overwrite a remote feature branch — it aborts if
  anyone else pushed to the same ref in the meantime, preventing the
  silent obliterate-someone-else's-work failure mode that bare `--force`
  enables.

### Scripts (`.claude/scripts/`)
- `py` — vetted Python entry point. Replaces the previous blanket
  `python3 -c *` permission with a fixed, auditable subcommand list
  (`json-pretty`, `json-check`, `yaml-check`, `hash`, `version`). To add
  a new use case, extend the script and review the change; never bypass
  the wrapper with `python3 -c "<arbitrary>"`.
- `gh-job-log` — fetches GitHub Actions logs for a failed run and
  surfaces only the failure-relevant tail. Usage:
  ```sh
  .claude/scripts/gh-job-log <run_id>          # whole run
  .claude/scripts/gh-job-log <run_id> <job_id> # single job
  ```
  Uses the `gh` CLI when available (`gh run view --log-failed`);
  otherwise falls back to the GitHub REST API via `curl` with
  `$GH_TOKEN` and `$GH_REPO` set. Writes the raw log to
  `.ai/artifacts/logs/gh-job-<run_id>-<ts>.log` and prints the
  last 50 lines of every failed step to stdout. Cited by the
  `log-volume` rule (`.claude/rules/log-volume.md`) as the canonical
  way to bring CI failures into the chat without pasting the full
  log.

### Project commands quick reference

**Rust** (workspace at `core/Cargo.toml`, toolchain pinned in
`core/rust-toolchain.toml`):

| Action | Command |
|---|---|
| Setup | `lefthook install` (or `just install-hooks`) |
| Check | `cargo check --manifest-path core/Cargo.toml --workspace` |
| Test (all) | `just test-rust` |
| Test (focused) | `cargo test -p <crate> --manifest-path core/Cargo.toml <test_name>` |
| Format | `just fmt-rust` |
| Lint / clippy | `just lint-rust` |
| Build | `just build-rust` |
| Reproduce CI | `cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test --workspace --locked` |

**Flutter/Dart** (`app/`, `sdk/liquid_sdk/`, `apps/*` — created incrementally):

| Action | Command |
|---|---|
| Get deps | `flutter pub get` (per package) |
| Analyze | `just lint-app` / `just lint-sdk` |
| Test (all) | `just test-app` / `just test-sdk` |
| Test (focused) | `cd app && flutter test <path>` |
| Integration | `cd app && flutter test integration_test/` |
| Format | `just fmt-app` / `just fmt-sdk` |
| Build | `just build-app target=<linux\|macos\|windows\|ios\|appbundle>` |

**CLI bats**: `just test-cli` (or `bats tests/cli/`).

**Combined**: `just test`, `just lint`, `just fmt`, `just check`.

### Delivery expectations
Before delivery:
- run the narrowest relevant tests during implementation
- run `cargo check` / focused `cargo test` for Rust changes
- run `flutter analyze` / focused `flutter test` for Dart changes (when the
  package exists)
- run `bats tests/cli/` for CLI changes
- review the git diff (use the `code-reviewer` subagent for non-trivial diffs)
- summarize changed files, test evidence, and risks

### Cloud notes
- Flutter UI validation is limited in cloud sessions when no
  emulator/simulator/browser is available — fall back to widget-level tests.
- Playwright MCP and Playwright CLI are not configured. Do not add them
  unless the repo gains real browser e2e tooling.
- No global MCP servers are configured beyond the GitHub integration.
- The Rust toolchain is pinned to `1.94.1` in `core/rust-toolchain.toml`
  *and* in `.github/workflows/ci.yml` (`dtolnay/rust-toolchain@master` with
  `toolchain: 1.94.1`). Bump both together; do not let CI drift to "stable".
