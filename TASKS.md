# Liquid — Task Queue

Active and upcoming implementation tasks. One task per heading.
Use `.github/ISSUE_TEMPLATE/task.md` to create new tasks via GitHub Issues.

Agents: read the task carefully, check the referenced milestone in
`IMPLEMENTATION_PLAN.md`, then invoke the `implement` skill.

---

## Active tasks

_None yet — project is in pre-scaffolding phase._

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
