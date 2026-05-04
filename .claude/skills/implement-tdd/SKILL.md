---
name: implement-tdd
description: Implement Rust and Flutter/Dart coding tasks with minimal changes, test-driven or test-near-change workflow, targeted verification, and artifact-aware context management. Use for feature implementation, bug fixes, refactors, and multi-step coding tasks where correctness and token efficiency matter.
---

# Implement TDD

A lightweight, token-efficient implementation workflow for Liquid.

For full Liquid project workflow (red/green, CLI-before-UI gate, E2E,
documentation update), the `implement` skill in `.claude/skills/implement/`
is the canonical procedure. Use this skill when you want a leaner loop.

## Workflow

1. Restate the objective in one short paragraph.
2. Inspect only the files needed to understand the change.
3. Identify the narrowest relevant verification command.
4. Create or update a failing test when behavior changes.
5. Implement the smallest viable change.
6. Run focused verification.
7. Iterate until focused verification passes.
8. Run broader verification when feasible.
9. Save long logs/artifacts under `.ai/artifacts/`.
10. Summarize changed files, tests run, and remaining risks.

## Rust verification preference

Use the narrowest applicable command:

- `cargo check --manifest-path core/Cargo.toml`
- `cargo test -p <crate> --manifest-path core/Cargo.toml <test_name>`
- `cargo test -p <crate> --manifest-path core/Cargo.toml`
- `cargo clippy --manifest-path core/Cargo.toml --all-targets -- -D warnings`
- `cargo fmt --manifest-path core/Cargo.toml --check`

Project shortcuts: `just test-rust`, `just lint-rust`, `just fmt-rust`,
`just build-rust`.

## Flutter/Dart verification preference

Run only when the affected package exists (the Flutter `app/`, `sdk/liquid_sdk/`,
and `apps/` trees are scaffolded incrementally per `IMPLEMENTATION_PLAN.md`).

- `cd app && flutter analyze`
- `cd app && flutter test <path>`
- `cd app && flutter test integration_test`
- `cd sdk/liquid_sdk && flutter test`

Project shortcuts: `just test-app`, `just test-sdk`, `just lint-app`,
`just lint-sdk`, `just fmt-app`, `just fmt-sdk`.

## CLI bats tests

- `bats tests/cli/<feature>.bats` (focused) or `bats tests/cli/` (suite).
- Project shortcut: `just test-cli`.

## Rules

- Do not perform broad rewrites.
- Do not read large files or logs into the main thread unnecessarily.
- Use the `test-triager` subagent for noisy Rust/Flutter logs.
- Use the `ui-validator` subagent for Flutter UI validation.
- Use the `code-reviewer` subagent before delivery for non-trivial diffs.
- Prefer deterministic shell filtering (`.claude/hooks/filter-test-output.sh`)
  over model-based log parsing.
- Keep the main thread focused on decisions, edits, and concise evidence.
- Honor project Absolute Rules from `CLAUDE.md`: no `unwrap()/expect()` outside
  tests, permission check first in every bridge function, every storage call
  takes a `WorkspaceId`, CLI before UI, failing test before implementation.

## Output format

At the end of each implementation phase, report:

- Change made:
- Files touched:
- Tests run:
- Result:
- Next step:
