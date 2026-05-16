---
name: implement
description: |
  Liquid project development workflow. AUTO-INVOKE at the start of every
  implementation task — feature, fix, CLI command, or UI widget. Enforces
  red/green TDD with hard gates, CLI-first validation before any UI work,
  E2E testing, project-Absolute-Rules review, and a documentation update
  pass. Optimized for high implementation quality and token efficiency.
  Do NOT invoke for documentation-only changes.
user-invocable: true
allowed-tools: Bash Read Edit Write TodoWrite
---

# implement — Liquid Development Workflow

The single canonical implementation workflow for Rust + Flutter/Dart work in
this repo. Every step is a **hard gate**: do not proceed until the gate is
green. Skipping a gate is a project policy violation.

Before touching any code, write down in one sentence:
- What layer(s) are affected (Rust crate / Dart SDK / Flutter widget / CLI)?
- Does this need UI? (If no, skip Steps 4–5.)
- Which milestone in `IMPLEMENTATION_PLAN.md` does this belong to?
- What is the **narrowest** verification command for this change?

Then execute every step below in order.

---

## Operating mode (token efficiency)

- Read only the files needed to understand the change. Do not preload large
  files. Use `rg`/`grep`/`Glob` for targeted lookups.
- **Log volume rule** (`.claude/rules/log-volume.md`): any command whose
  stdout + stderr is expected to exceed ~50 lines MUST be routed through
  `.claude/hooks/filter-test-output.sh` (or the `test-triager` subagent,
  or — for CI logs — `.claude/scripts/gh-job-log`). Examples:
  ```sh
  cargo test 2>&1 | .claude/hooks/filter-test-output.sh
  flutter test 2>&1 | .claude/hooks/filter-test-output.sh
  bats tests/cli/ 2>&1 | .claude/hooks/filter-test-output.sh
  ```
  Raw logs land in `.ai/artifacts/logs/`; only a compact summary reaches the
  main thread.
- Delegate noisy investigation to subagents:
  - `test-triager` for cargo/flutter/bats/analyzer log analysis.
  - `ui-validator` for Flutter widget/integration/golden validation.
  - `code-reviewer` for diff review (Step 6).
- Save large artifacts (screenshots, golden diffs, traces) under
  `.ai/artifacts/{logs,ui,diffs}/`. Never paste them into chat.
- Run the **narrowest** test first; escalate only after focused tests pass.

---

## Step 1 — Red: write failing tests first

Write tests BEFORE implementation. Confirm they fail. The failing test output
is your specification.

```sh
# Rust crate
cargo test -p <crate> --manifest-path core/Cargo.toml <test_name>
# must print FAILED, not compile errors

# Dart SDK
cd sdk/liquid_sdk && flutter test test/<path>_test.dart
# must print test failures

# CLI
bats tests/cli/<feature>.bats
# must print failures or "command not found"
```

**Hard gate:** Do not proceed until you have at least one *red* test per
affected layer that fails for the right reason (assertion failure, not
compile error or import error).

---

## Step 2 — Green: minimum code to pass

Write only what the failing tests require. No extra abstractions, no
future-proofing, no "while I'm here" cleanup.

**Before the first call site:** grep the actual API signature
(`grep -nE 'pub (fn|trait) <name>' core/<crate>/src/`) — see
[`.claude/rules/api-grep-discipline.md`](../../rules/api-grep-discipline.md).
Assumed signatures cost 3–5 edit rounds each; two minutes of grep
replaces ten minutes of clippy / cargo-test ping-pong.

```sh
cargo test -p <crate> --manifest-path core/Cargo.toml   # all green
cd sdk/liquid_sdk && flutter test                        # all green
bats tests/cli/<feature>.bats                            # all green
```

**Hard gate:** If you wrote code no test exercises, delete it before moving on.

---

## Step 3 — CLI validation gate (mandatory before any UI)

Per `CLAUDE.md` Absolute Rule 6: **CLI before UI.** Prove the feature works
end-to-end via CLI. An agent must be able to exercise the full feature using
only `liquid` commands and `jq`.

Checklist:
- [ ] Happy path passes
- [ ] Auth check: a principal without permission is rejected with exit code ≠ 0
- [ ] Error path: invalid input → meaningful error on stderr
- [ ] `--format json` returns valid newline-delimited JSON
- [ ] New command documented in `IMPLEMENTATION_PLAN.md §12`

```sh
just test-cli   # full bats suite, not just the new test
```

**Hard gate:** Do NOT proceed to Step 4 until every box is checked.

---

## Step 4 — UI implementation (skip if feature has no UI)

Only after Step 3 is fully green:

- Implement widget(s) in `app/lib/`
- Use `AsyncNotifierProvider` (Riverpod) for every FFI-backed state
- **No business logic in Dart** — only in Rust via FFI (Absolute Rule per
  CLAUDE.md/IMPLEMENTATION_PLAN.md §1)
- Write widget tests in `app/test/widget/`

```sh
cd app && flutter test
just lint-app
```

For UI validation, use the `ui-validator` subagent. In cloud Claude Code
sessions without an emulator/simulator/desktop runtime, fall back to
widget-level tests and report the limitation explicitly.

---

## Step 5 — E2E validation (skip if feature has no UI)

Add or update an integration test in `app/integration_test/` covering the
critical user path from app launch to final state assertion. Use `patrol` for
gesture-heavy flows (drag, long-press, swipe).

```sh
cd app && flutter test integration_test/
```

**Hard gate:** UI features without a passing integration test are not done.

---

## Step 6 — Review pass (never skip)

```sh
# Rust quality
cargo fmt --manifest-path core/Cargo.toml --check
cargo clippy --manifest-path core/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path core/Cargo.toml --workspace

# Dart / Flutter quality (when packages exist)
just lint-app
just lint-sdk
just test-app
just test-sdk

# CLI
just test-cli
```

Or, equivalently, the full pre-push validation: `just check`.

**Manual diff review checklist** (project Absolute Rules — see `CLAUDE.md`):
- [ ] No `unwrap()` / `expect()` outside `#[cfg(test)]` (Absolute Rule 1)
- [ ] No `dart:io`, no platform plugins in `apps/` or `sdk/` (Absolute Rule 2)
- [ ] No direct Dart references between components — go through `SlotBroker`
      (Absolute Rule 3)
- [ ] Permission check is the first line of every new bridge function
      (Absolute Rule 4)
- [ ] Every new storage call takes a `WorkspaceId` (Absolute Rule 5)
- [ ] No abstraction serving only one callsite
- [ ] No duplicated logic from an existing crate or SDK class
- [ ] Hot paths are O(1) or O(log n)

For non-trivial diffs, delegate to the `code-reviewer` subagent. Fix every
finding before committing.

---

## Step 7 — Documentation update

- [ ] `IMPLEMENTATION_PLAN.md §4/§9` — if trait signatures changed
- [ ] `IMPLEMENTATION_PLAN.md §12` — if CLI grammar changed
- [ ] `IMPLEMENTATION_PLAN.md §11` + `docs/sdk-guide/` — if SDK API changed
- [ ] `IMPLEMENTATION_PLAN.md §17` — if a pre-1.0 obligation just became binding
- [ ] `docs/adr/NNN-title.md` — if a design decision was made (use `TEMPLATE.md`)
- [ ] `CHANGELOG.md` — if user-visible behaviour changed (under `## [Unreleased]`)
- [ ] `README.md` Status table + `TASKS.md` — if a milestone moved Planned → Done
- [ ] `README.md` (Vision / Why / Quickstart) — only if the user-visible *concept* changed
- [ ] `DEVELOPER_INFO.md` — if architecture or design rationale changed
- [ ] **Run the `sync-docs` skill** as the final audit pass; resolve every
      reported drift before committing

---

## Output format (end of each phase)

Report:

- **Change made:** one sentence
- **Files touched:** path list
- **Tests run:** `command` → result (focused → broader)
- **Artifacts:** any `.ai/artifacts/...` paths worth referencing
- **Gates passed:** Step 1 ☑ / Step 2 ☑ / Step 3 ☑ / ...
- **Next step:** the smallest next action

---

## Commit

Format: `<type>(<scope>): <summary>` (Conventional Commits — see `CLAUDE.md`).
One logical change per commit. Push to the current feature branch only after
all relevant gates are green.
