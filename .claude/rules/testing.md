# Testing Rules

- Prefer TDD or test-near-change for behavior changes.
- Add or update tests for changed behavior.
- Run the narrowest relevant test first.
- Escalate to broader tests only after focused tests pass.
- Store long logs under `.ai/artifacts/logs/`.
- Summarize failures with:
  - command
  - failing test
  - first meaningful error
  - likely root cause
  - next smallest action
- Do not paste full logs into chat unless explicitly requested.

## Rust-specific testing

- Prefer `cargo test -p <crate> --manifest-path core/Cargo.toml <test_name>` for focused tests.
- Use `cargo check --manifest-path core/Cargo.toml` before broad test runs when type errors are likely.
- Use `cargo clippy --manifest-path core/Cargo.toml --all-targets -- -D warnings` for quality checks when feasible.
- Workspace lints already deny `unsafe_code` and warn on `unwrap`/`expect`/`panic`/`todo`/`unimplemented` (see `core/Cargo.toml`). Do not add `unwrap()`/`expect()` outside `#[cfg(test)]` — see `CLAUDE.md` Absolute Rules.
- Preserve existing feature flags and the `core/` workspace layout.
- Project shortcut: `just test-rust`, `just lint-rust`, `just fmt-rust`.

## Flutter/Dart-specific testing

The `app/` and `sdk/liquid_sdk/` directories are scaffolded incrementally
(see `IMPLEMENTATION_PLAN.md`). Run Flutter commands only when the relevant
package exists.

- Prefer `flutter test <path>` for focused widget/unit tests.
- Use `flutter analyze` (or `dart analyze`) before broad changes are considered done.
- For widget behavior, prefer widget tests in `app/test/widget/` or `sdk/liquid_sdk/test/`.
- For end-to-end app behavior, prefer the existing `integration_test/` setup
  (`patrol` is the project's chosen wrapper for gesture-heavy flows).
- For visual behavior, use existing golden/screenshot tooling if present.
- Project shortcuts: `just test-app`, `just test-sdk`, `just lint-app`, `just lint-sdk`.

## CLI bats tests

- Located at `tests/cli/`. Run with `bats tests/cli/` (or `just test-cli`).
- Per `CLAUDE.md` Absolute Rule 6, the CLI must be proven before any UI work.
