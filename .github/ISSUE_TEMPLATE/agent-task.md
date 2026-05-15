---
name: Agent task
about: A scoped task intended to be picked up and executed by a Claude Code agent.
title: "[AGENT-TASK] "
labels: ["task", "agent"]
assignees: ''
---

<!--
  This template is for tasks specifically intended for AI agents to
  pick up and complete. It is more prescriptive than the generic
  "Implementation task" template — the agent reads this issue cold and
  must produce a passing PR without further clarification.

  Before submitting:
    1. Pick the milestone from IMPLEMENTATION_PLAN.md §5–§8.
    2. Fill in *every* section. Empty sections cause re-prompts.
    3. The "acceptance" section is the spec — be specific enough that
       success is binary.
-->

## Milestone

**Phase:** 1 | 2 | 3 | 4
**Milestone:** M<!-- number(.subnumber) --> — <!-- title, e.g. "Minimal agent CLI" -->
**Section:** IMPLEMENTATION_PLAN.md §<!-- e.g. 5.7 -->

## Layer(s) affected

Tick every layer the change touches. Leave the rest unchecked — they
will be skipped by `lefthook` and CI.

- [ ] `core/liquid-core`
- [ ] `core/liquid-vcs`
- [ ] `core/liquid-auth`
- [ ] `core/liquid-permissions`
- [ ] `core/liquid-cache`
- [ ] `core/liquid-bindings`
- [ ] `core/liquid-sdk-bridge`
- [ ] `core/liquid-cli`
- [ ] `sdk/liquid_sdk`
- [ ] `app/` (Flutter shell)
- [ ] `apps/<name>` (first-party app)
- [ ] `tests/cli/` (bats)
- [ ] Docs only (no code change)

## What

<!--
  One paragraph: what the agent should produce, and why it matters
  for the milestone. State the user-visible behaviour change in
  agent terms ("after this, `liquid page write` succeeds when
  invoked with a valid AppEditor token") — not implementation terms
  ("add a function to bridge.rs"). The agent decides the
  implementation; the human decides the behaviour.
-->

## Tests to add (TDD red-first)

<!--
  The Liquid `implement` skill (.claude/skills/implement/SKILL.md)
  enforces a red-first test gate before any implementation code.
  List the tests you expect the agent to add. Be specific:

    - Test file path:
    - Test function names + what they assert:

  At least ONE failing test per affected layer.
-->

- [ ] Rust unit:  `core/liquid-XXX/src/…` — `#[test] fn …`
- [ ] Rust integration: `core/liquid-XXX/tests/…`
- [ ] Flutter widget: `sdk/liquid_sdk/test/…` or `app/test/widget/…`
- [ ] CLI bats: `tests/cli/<feature>.bats`
- [ ] Integration / E2E: `app/integration_test/…` (UI changes only)

## Docs to update

<!--
  The `implement` skill's Step 7 requires every behaviour-change
  commit to land with matching doc updates. List what changes here so
  the agent does not skip a surface. Empty list = no doc update is
  expected (review-pass will catch the mistake if there is one).
-->

- [ ] `IMPLEMENTATION_PLAN.md` §4 / §9 (trait signature)
- [ ] `IMPLEMENTATION_PLAN.md` §11 + `docs/sdk-guide/` (SDK API)
- [ ] `IMPLEMENTATION_PLAN.md` §12 (CLI grammar)
- [ ] `IMPLEMENTATION_PLAN.md` §5.N (milestone status)
- [ ] `IMPLEMENTATION_PLAN.md` §17 (pre-1.0 obligation became binding)
- [ ] `docs/adr/NNN-title.md` (new ADR — extends or contradicts an existing decision)
- [ ] `docs/security/threat-model.md` (new principal, surface, or trust boundary)
- [ ] `README.md` status table (milestone moved Planned → Done)
- [ ] `TASKS.md` (move from Active → Done section)
- [ ] `CHANGELOG.md` `## [Unreleased]` (user-visible behaviour)

## Acceptance criteria

<!--
  The agent considers the task done when *every* box below is ticked.
  Add task-specific lines below the boilerplate.
-->

- [ ] Failing test landed and confirmed red on the agent's branch
      (`implement` Step 1 gate).
- [ ] All affected-layer tests pass on the agent's branch.
- [ ] `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings`
      pass for the Rust layer.
- [ ] `flutter analyze` + `dart format --output=none --set-exit-if-changed`
      pass for any Flutter layer touched.
- [ ] `bats tests/cli/` passes for any CLI change.
- [ ] No `unwrap()` / `expect()` outside `#[cfg(test)]` (Absolute Rule 1).
- [ ] No `dart:io` / no platform plugins in `apps/` or `sdk/` (Absolute Rule 2).
- [ ] `require_permission!` is the first line of every new
      `liquid-sdk-bridge` fn (Absolute Rule 4).
- [ ] Every new storage call carries a `WorkspaceId` (Absolute Rule 5).
- [ ] If the change has any data path: CLI exercises that path before
      any UI work (Absolute Rule 6).
- [ ] Docs updated per the checklist above.
- [ ] `sync-docs` skill reports no critical drift.
- [ ] `code-reviewer` subagent's findings are addressed.
- [ ] PR opened with title `<type>(<scope>): <summary>` (Conventional
      Commits) and links this issue.

### Task-specific acceptance

<!--
  Add the binary success criteria specific to this task. Examples:
    - `liquid workspace create demo-ws` returns exit 0 and prints
      a UUID on stdout.
    - `liquid page write /pages/x --data '{"k":1}'` round-trips
      via `liquid page read`.
    - The new Rust trait `Foo` has at least 3 implementations
      under test and exposes a single `LiquidError` error variant.
-->

- [ ] …

## Affected files (best-effort)

<!--
  The agent will revise this list as it works. Provide the starting
  points so the agent does not have to grep the world.
-->

- `core/<crate>/src/<file>.rs`
- `app/lib/<file>.dart`
- `tests/cli/<feature>.bats`

## Constraints / prior art

<!--
  Anything the agent should know before starting:
    - related ADRs (link them);
    - decisions already taken upstream;
    - performance budgets ("must be O(log n) per workspace");
    - blocking tasks (`Blocks: TASK-NNN` / `Blocked by: TASK-NNN`).
-->
