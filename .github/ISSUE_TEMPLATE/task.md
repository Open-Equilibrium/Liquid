---
name: Implementation task
about: A scoped implementation task for agents or developers
title: "[TASK] "
labels: task
assignees: ''
---

## What

<!-- One paragraph: what changes and why. -->

## Phase & Milestone

**Phase:** 1 | 2 | 3 | 4
**Milestone:** M<!-- number --> — <!-- title from IMPLEMENTATION_PLAN.md -->

## Acceptance criteria

- [ ] Failing test written and confirmed red
- [ ] Tests pass green
- [ ] CLI validates the feature end-to-end (`bats tests/cli/`)
- [ ] UI implemented with widget tests (if applicable)
- [ ] E2E integration test passes (if UI involved)
- [ ] Review pass clean (clippy -D warnings, flutter analyze, no `unwrap()`, no platform imports)
- [ ] Docs updated (IMPLEMENTATION_PLAN.md, sdk-guide/, ADR if a new design decision)

## Affected files

<!-- List the crates, packages, and files this task will modify. -->

- `core/`
- `app/lib/`
- `sdk/liquid_sdk/lib/`

## Notes

<!-- Constraints, dependencies on other tasks, or prior art. -->
