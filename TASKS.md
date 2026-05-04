# Liquid — Task Queue

Active and upcoming implementation tasks. One task per heading.
Use `.github/ISSUE_TEMPLATE/task.md` to create new tasks via GitHub Issues.

Agents: read the task carefully, check the referenced milestone in
`IMPLEMENTATION_PLAN.md`, then invoke the `implement` skill.

---

## Active tasks

### [TASK-002] `ContentStore` trait + `InMemoryContentStore`

**Phase:** 1
**Milestone:** M2 (IMPLEMENTATION_PLAN.md §5.2, sub-tasks 1 & 3 of 4)
**Status:** Done

**What.** Define the `ContentStore` trait in `liquid-vcs` (per §4.1) and ship
`InMemoryContentStore` — the test/dev backend that satisfies the trait without
any Jujutsu dependency. Includes a typed `Operation` log with `Create | Update
| Delete | Undo` variants and proper undo semantics. Trait error type
normalises to `LiquidError` (the §4.1 spec used `StoreError`; we reconcile to
the workspace-wide error type so cross-crate boundaries stay uniform).

**Acceptance criteria.**
- [x] `cargo test -p liquid-vcs` is green (12 tests incl. M2 plan-level criterion)
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` clean
- [x] No `unwrap()`/`expect()` outside `#[cfg(test)]`
- [x] `InMemoryContentStore` is `Send + Sync`
- [x] `IMPLEMENTATION_PLAN.md` §4.1 updated to reflect the trait signature actually shipped

### [TASK-003] `JujutsuContentStore` + ADR-001 (jj-lib pinning)

**Phase:** 1
**Milestone:** M2 (IMPLEMENTATION_PLAN.md §5.2, sub-tasks 2 & 4 of 4)
**Status:** Planned
**Blocked by:** TASK-002

**What.** Add `jj-lib` as a pinned dependency, implement `JujutsuContentStore`
satisfying the `ContentStore` trait against a real Jujutsu repository under
`~/.liquid/workspaces/<id>/`, and write ADR-001 (`docs/adr/001-jujutsu-pinning.md`)
documenting the pinned version and migration policy.

---

## Done tasks

### [TASK-001] Rust workspace bootstrap + `liquid-core` primitives

**Phase:** 1
**Milestone:** M1 (IMPLEMENTATION_PLAN.md §5.1)
**Status:** Done

**What.** Create the `core/` Cargo workspace with stubs for all eight crates
(`liquid-core`, `liquid-vcs`, `liquid-auth`, `liquid-permissions`,
`liquid-cache`, `liquid-bindings`, `liquid-sdk-bridge`, `liquid-cli`) and
fully implement `liquid-core`: ID newtypes, `PrincipalId`, `ContentHash`,
`StorePath`, `SlotName`, `SlotValue`, `Action`, `Resource`, `TenantConfig`,
`LiquidError`. Every public function returns `Result<_, LiquidError>`; no
`unwrap()`/`expect()` outside `#[cfg(test)]`.

**Acceptance criteria.**
- [x] `cargo test -p liquid-core` is green (26 tests)
- [x] `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` clean
- [x] No `unwrap()`/`expect()` outside `#[cfg(test)]`
- [x] Every ID type has construction, equality, and serde round-trip tests
- [x] `StorePath` rejects `..`, absolute paths, empty segments
- [x] `SlotName` rejects malformed names
- [x] `ContentHash::from_hex` validates length and lowercase-hex
- [ ] CI's `detect` job picks up `core/Cargo.toml` and runs the rust matrix (verified post-push)

---

## Task template

Copy this block when adding a task directly to this file:

```markdown
## [TASK-NNN] Short title

**Phase:** 1 | 2 | 3 | 4
**Milestone:** M1–M20 (IMPLEMENTATION_PLAN.md reference)
**Status:** Planned | In Progress | Blocked | Done
**Blocked by:** TASK-NNN (if applicable)

### What
One paragraph describing the change and why it is needed.

### Acceptance criteria
- [ ] Failing test written and confirmed red
- [ ] Tests pass green
- [ ] CLI validates the feature end-to-end (bats tests/cli/)
- [ ] UI implemented (if applicable) with widget tests
- [ ] E2E integration test passes (if UI involved)
- [ ] Review pass clean (clippy, analyze, no unwrap, no platform imports)
- [ ] Docs updated (IMPLEMENTATION_PLAN.md, sdk-guide/, ADR if needed)

### Affected files
- `core/<crate>/src/`
- `app/lib/`
- `sdk/liquid_sdk/lib/`

### Notes
Any constraints, edge cases, or prior art worth knowing.
```
