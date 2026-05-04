---
name: deliver
description: Prepare a completed Rust and Flutter/Dart coding task for delivery with final verification, concise diff review, risk summary, and PR-ready notes. Use when implementation is complete or the user asks for final summary, delivery, PR description, or handoff.
---

# Deliver

Prepare a completed task for handoff.

## Checklist

1. Inspect `git status`.
2. Inspect `git diff --stat`.
3. Inspect the focused diff.
4. Run relevant final verification if feasible.
5. Use the `code-reviewer` subagent for non-trivial diffs.
6. Summarize test evidence.
7. Summarize risks and follow-ups.
8. Provide PR-ready notes.

## Verification preference

For Rust changes, prefer:
- focused `cargo test -p <crate> --manifest-path core/Cargo.toml`
- `cargo check --manifest-path core/Cargo.toml`
- `cargo clippy --manifest-path core/Cargo.toml --all-targets -- -D warnings`
- `cargo fmt --manifest-path core/Cargo.toml --check`
- or `just lint-rust && just test-rust`

For Flutter/Dart changes (when the package exists), prefer:
- `flutter analyze` / `dart analyze`
- focused `flutter test <path>`
- widget/integration/golden tests if relevant
- build commands only when necessary or already part of the project workflow

For full pre-push validation: `just check` (matches CI in `.github/workflows/ci.yml`).

## Output format

Return:

## Summary
- ...

## Files changed
- `path`: reason

## Verification
- `command`: result

## Review notes
- Critical:
- Warnings:
- Follow-ups:

## Suggested PR description
Title: <type>(<scope>): <summary>      # Conventional Commits — see CLAUDE.md
Body:

Do not commit or push unless explicitly asked.
