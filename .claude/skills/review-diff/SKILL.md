---
name: review-diff
description: Review the current Rust and Flutter/Dart git diff for correctness, tests, security, performance, stability, idiomatic code, and minimality. Use after implementation phases, before delivery, or when asked to review changes.
---

# Review Diff

## Scope

Review only:
- the current `git diff --stat` and `git diff`
- directly relevant surrounding code
- tests related to changed behavior

## Checks

General:
- Correctness
- Security
- Performance
- Stability
- Test coverage
- Minimality
- Backwards compatibility
- Error handling

Rust-specific:
- ownership/lifetime complexity
- unnecessary clones/allocations
- panic risk (`unwrap()`/`expect()` outside `#[cfg(test)]` is forbidden — Absolute Rule 1)
- error handling via `Result` / `thiserror`
- async/runtime correctness (`tokio`)
- serde/schema compatibility
- public API compatibility across `liquid-*` crates
- unsafe usage (workspace forbids `unsafe_code`)
- permission check is the first line of every bridge function (Absolute Rule 4)
- every storage call takes a `WorkspaceId` (Absolute Rule 5)

Flutter/Dart-specific:
- state-management consistency (Riverpod / `AsyncNotifierProvider`)
- widget rebuild performance
- async lifecycle safety
- navigation/routing correctness
- platform-specific risk
- localization/theming/accessibility impact
- analyzer/test coverage
- `no_platform_imports` and `no_cross_component_reference` lint compliance

## Rules

- Prefer concrete findings over general advice.
- Cite file paths and relevant functions/lines where possible.
- Do not paste full diffs.
- Do not edit files unless the user asks for fixes.
- If the diff is large, delegate to the `code-reviewer` subagent.

## Output format

Return JSON:

```
{
  "summary": "...",
  "critical": [],
  "warnings": [],
  "suggestions": [],
  "missing_tests": [],
  "evidence": [],
  "recommended_next_step": "...",
  "confidence": "high|medium|low"
}
```
