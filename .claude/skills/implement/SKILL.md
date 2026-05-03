---
name: implement
description: |
  Liquid project development workflow. AUTO-INVOKE at the start of every
  implementation task — feature, fix, CLI command, or UI widget. Enforces
  TDD (red/green), CLI-first validation before any UI work, E2E testing,
  and a KPI review pass. Do NOT invoke for documentation-only changes.
user-invocable: true
allowed-tools: Bash Read Edit Write TodoWrite
---

# implement — Liquid Development Workflow

Before touching any code, write down in one sentence:
- What layer(s) are affected (Rust crate / Dart SDK / Flutter widget / CLI)?
- Does this need UI? (If no, skip Steps 4–5.)
- Which milestone in IMPLEMENTATION_PLAN.md does this belong to?

Then execute every step below in order. Do not skip.

---

## Step 1 — Red: write failing tests first

Write tests BEFORE implementation code. Confirm they fail.

```sh
# Rust crate
cargo test -p <crate> --manifest-path core/Cargo.toml
# must print FAILED, not compile errors

# Dart SDK
cd sdk/liquid_sdk && flutter test
# must print test failures

# CLI
bats tests/cli/<feature>.bats
# must print failures or "command not found"
```

Stop here until you have at least one red test per affected layer.

---

## Step 2 — Green: implement minimum code

Write only what the failing tests require.

```sh
cargo test -p <crate> --manifest-path core/Cargo.toml  # all green
cd sdk/liquid_sdk && flutter test                        # all green
bats tests/cli/<feature>.bats                           # all green
```

If you wrote code no test exercises, delete it.

---

## Step 3 — CLI validation gate (mandatory before any UI)

Prove the feature works end-to-end via CLI. An agent must be able to exercise
the full feature using only `liquid` commands and `jq`.

Checklist:
- [ ] Happy path passes
- [ ] Auth check: a principal without permission is rejected with exit code ≠ 0
- [ ] Error path: invalid input → meaningful error on stderr
- [ ] `--format json` returns valid newline-delimited JSON
- [ ] New command documented in IMPLEMENTATION_PLAN.md §12

```sh
bats tests/cli/   # full suite, not just the new test
```

Do NOT proceed to Step 4 until this is fully green.

---

## Step 4 — UI implementation (skip if feature has no UI)

- Implement widget(s) in `app/lib/`
- `AsyncNotifierProvider` for every FFI-backed state
- No business logic in Dart — only in Rust via FFI
- Write widget tests in `app/test/widget/`

```sh
cd app && flutter test
```

---

## Step 5 — E2E validation (skip if feature has no UI)

Add or update an integration test in `app/integration_test/` covering the
critical user path from app launch to final state assertion.

```sh
cd app && flutter test integration_test/
```

Use `patrol` for flows involving gestures (drag, long-press, swipe).

---

## Step 6 — Review pass (never skip)

```sh
# Rust quality
cargo fmt --manifest-path core/Cargo.toml --check
cargo clippy --manifest-path core/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path core/Cargo.toml --workspace

# Dart / Flutter quality
dart format --output=none --set-exit-if-changed app/
flutter analyze --project app
dart format --output=none --set-exit-if-changed sdk/liquid_sdk/
flutter analyze --project sdk/liquid_sdk
flutter test app/

# CLI
bats tests/cli/
```

Manual diff review checklist:
- [ ] No `unwrap()` / `expect()` outside `#[cfg(test)]`
- [ ] No `dart:io`, no platform plugins in `apps/` or `sdk/`
- [ ] No business logic in Dart (bridge calls only)
- [ ] Permission check is the first line of every new bridge function
- [ ] Every new storage call takes a `WorkspaceId`
- [ ] No abstraction serving only one callsite
- [ ] No duplicated logic from an existing crate or SDK class
- [ ] Hot paths are O(1) or O(log n)

Fix every issue before committing.

---

## Step 7 — Documentation update

- [ ] `IMPLEMENTATION_PLAN.md §4/§9` — if trait signatures changed
- [ ] `IMPLEMENTATION_PLAN.md §12` — if CLI grammar changed
- [ ] `IMPLEMENTATION_PLAN.md §11` + `docs/sdk-guide/` — if SDK API changed
- [ ] `docs/adr/NNN-title.md` — if a design decision was made (use TEMPLATE.md)
- [ ] `README.md` — only if the user-visible concept changed

---

## Commit

Format: `<type>(<scope>): <summary>` (Conventional Commits — see CLAUDE.md).
One logical change per commit. Push to the current feature branch.
