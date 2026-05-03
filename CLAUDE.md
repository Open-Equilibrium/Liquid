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
- If the README concept changed: update `README.md`.

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

## How to Install Git Hooks

```sh
git config core.hooksPath .githooks
```

Run this once after cloning. The hooks enforce formatting and linting on every commit and push.

---

## Running the Full Stack Locally

```sh
# Build Rust core
cargo build --workspace

# Generate Dart FFI bindings
cd core && cargo run -p flutter_rust_bridge_codegen -- generate

# Run Flutter app (desktop)
cd app && flutter run -d linux   # or macos / windows

# Run agent CLI
cargo run -p liquid-cli -- --help
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
