# /implement — Feature Implementation Workflow

Execute the Liquid development workflow for the feature described in `$ARGUMENTS`.

You MUST follow every step in order. Do not skip steps.
Read CLAUDE.md for the full rationale behind each rule.

---

## Understand the scope

1. Re-read the relevant section(s) of `IMPLEMENTATION_PLAN.md` that cover this feature.
2. Identify which crate(s), SDK class(es), or Flutter widget(s) are affected.
3. Identify whether this feature requires a new CLI command, a new SDK API, a new UI widget,
   or all three. Write this down before proceeding.

---

## Step 1 — Red: write failing tests

For each affected layer, write tests BEFORE any implementation code.

**Rust crate:**
```sh
# Write tests in core/<crate>/src/lib.rs or core/<crate>/tests/
cargo test -p <crate>  # must show test failures, not compile errors
```

**Dart SDK:**
```sh
# Write tests in sdk/liquid_sdk/test/
cd sdk/liquid_sdk && flutter test  # must show test failures
```

**CLI:**
```sh
# Write bats tests in tests/cli/<feature>.bats
bats tests/cli/<feature>.bats  # must show failures
```

Do not proceed until you have at least one red test per affected layer.

---

## Step 2 — Green: implement minimum code

Implement only what is needed to make the failing tests pass.

```sh
cargo test -p <crate>           # all Rust tests green
cd sdk/liquid_sdk && flutter test  # all SDK tests green
bats tests/cli/<feature>.bats  # all CLI tests green
```

No extra abstractions. If you find yourself writing code that no test exercises, delete it.

---

## Step 3 — CLI validation

Prove the feature works end-to-end via CLI before touching Flutter.

Write a shell script or expand the bats suite to cover:
- [ ] Happy path: feature works as documented
- [ ] Auth check: an agent without permission is rejected
- [ ] Error path: invalid input returns a meaningful error to stderr and exit code ≠ 0
- [ ] Output format: `--format json` returns valid JSON; `--format text` is human-readable

```sh
bats tests/cli/
```

All tests must be green. If the CLI cannot exercise the full feature, the data model or FFI
surface is incomplete — fix it before proceeding.

---

## Step 4 — UI implementation (only if feature has a UI component)

If the feature is CLI-only (agent workflows, background jobs), skip this step.

- Implement Flutter widget(s) in `app/lib/`
- Use `AsyncNotifierProvider` for FFI-backed state
- Write widget tests in `app/test/widget/`

```sh
cd app && flutter test
```

---

## Step 5 — E2E validation (only if feature has a UI component)

Add or update an integration test in `app/integration_test/` that covers the critical path.

```sh
cd app && flutter test integration_test/
```

The test must:
- Start from a real app launch (no mocks)
- Exercise the new UI interaction
- Assert the final state matches expected output

---

## Step 6 — Review pass

Run the full quality suite:

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test --workspace
dart format --output=none --set-exit-if-changed .
flutter analyze
flutter test
bats tests/cli/
```

Then review your diff for:
- [ ] No `unwrap()` / `expect()` outside tests
- [ ] No platform imports in `apps/` or `sdk/`
- [ ] No business logic in Dart (only in Rust via FFI)
- [ ] Permission check is first in every new bridge function
- [ ] Every new storage call takes a `WorkspaceId`
- [ ] No redundant abstractions or duplicated logic
- [ ] Hot paths are O(1) or O(log n) where feasible

Fix every issue found before committing.

---

## Step 7 — Documentation

Update every affected document:

- [ ] `IMPLEMENTATION_PLAN.md` — if trait signatures, CLI grammar, or SDK APIs changed
- [ ] `docs/sdk-guide/` — if a public SDK API was added or changed
- [ ] `docs/adr/` — if a design decision was made; use `docs/adr/TEMPLATE.md`
- [ ] `README.md` — only if the user-visible concept changed

---

## Commit

Follow Conventional Commits format (see CLAUDE.md). One logical change per commit.
Push to the current feature branch.
