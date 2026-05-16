<!--
Thank you for contributing to Liquid! Please fill in the sections below.
The reviewer checklist at the bottom is the same one CONTRIBUTING.md asks
reviewers to use — pre-filling it makes review faster.
-->

## Summary

<!-- One short paragraph: what does this PR do, and why? -->

## Related issue / task

<!-- Link the GitHub issue, TASK-NNN entry, or IMPLEMENTATION_PLAN.md
     milestone this PR addresses. PRs without a linked task that change
     more than docs need maintainer pre-approval (see CONTRIBUTING.md). -->

Closes #
Implements: TASK-

## What changed

<!-- Bullet list of the user-visible / API-visible changes. Internal
     refactors that don't change behaviour can be summarised in one line. -->

-

## Test plan

<!-- The exact commands a reviewer can run to verify this change locally. -->

```sh
cargo test --manifest-path core/Cargo.toml --workspace
cargo clippy --manifest-path core/Cargo.toml --workspace --all-targets -- -D warnings
cargo fmt --manifest-path core/Cargo.toml --check
# add Flutter / bats commands once the relevant layers exist
```

## Screenshots / logs

<!-- For UI changes: before/after screenshots or recordings.
     For bug fixes: the failing → passing log excerpt.
     Skip this section if it doesn't apply. -->

## Author checklist

- [ ] **Failing test first.** I wrote a test for the new behaviour
      *before* the implementation, and confirmed it failed without
      the implementation.
- [ ] **Tests now pass.** `cargo test` is green for every affected
      crate (and Flutter / bats when applicable).
- [ ] **Lint clean.** `cargo fmt --check` and
      `cargo clippy --all-targets -- -D warnings` pass.
- [ ] **No `unwrap()` / `expect()` outside `#[cfg(test)]`** (project
      Absolute Rule 1).
- [ ] **Permission gate.** Any new `liquid-sdk-bridge` FFI function
      starts with `require_permission!` (Absolute Rule 4).
- [ ] **Workspace-scoped storage.** Any new storage call carries a
      `WorkspaceId` (Absolute Rule 5).
- [ ] **No platform imports** in `apps/` or `sdk/` (Absolute Rule 2).
- [ ] **No cross-component references**; all cross-component
      communication goes through `SlotBroker` (Absolute Rule 3).
- [ ] **CLI before UI.** If this change touches data, it is reachable
      from the `liquid` CLI before any UI work begins (Absolute Rule 6).
- [ ] **Coverage claim.** Any "CLI integration test added" claim above
      is annotated as either *live* (test actually runs assertions
      against the new behaviour) or *skip-pending-M6.5* (spec
      scaffold only, every step is `skip`). See `tests/cli/README.md`.
- [ ] **Conventional Commit message** with an appropriate scope
      (`feat(vcs):`, `fix(permissions):`, `docs:`, …).
- [ ] **Docs updated.** I have updated, where relevant:
  - [ ] `IMPLEMENTATION_PLAN.md` (§4 / §9 / §11 / §12)
  - [ ] `CHANGELOG.md` under `## [Unreleased]`
  - [ ] `docs/adr/` if a design decision contradicts or extends an ADR
  - [ ] `docs/sdk-guide/` if a public SDK API changed
- [ ] **Single logical change.** I am not bundling drive-by refactors.

## Reviewer checklist

- [ ] **Correctness:** the test exercises the new behaviour.
- [ ] **Project rules:** none of the seven Absolute Rules violated.
- [ ] **Scope:** PR does one thing.
- [ ] **Bloat:** no abstraction serving exactly one callsite; no
      duplicated logic from another crate.
- [ ] **Stability:** no `unwrap()` / `expect()` outside `#[cfg(test)]`;
      no input reaching storage without validation.
- [ ] **Performance:** hot paths are O(1) or O(log n) where reasonable.
- [ ] **Docs:** user-visible / API-visible changes are documented.

<!-- For non-trivial diffs the maintainers may run the
     `code-reviewer` subagent (.claude/agents/code-reviewer.md)
     against this PR. -->
