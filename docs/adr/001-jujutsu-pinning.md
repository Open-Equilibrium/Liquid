# ADR-001 — VCS persistence: filesystem stand-in for Phase 1, `jj-lib` deferred

**Status:** Accepted
**Date:** 2026-05-04
**Deciders:** Claude (M2 implementer), repository maintainers

## Context

`IMPLEMENTATION_PLAN.md` §4.1 defines the `ContentStore` trait, and §5.2
(Milestone 2) calls for two concrete implementations:

1. `InMemoryContentStore` — test/dev backend with no persistence (shipped in TASK-002).
2. `JujutsuContentStore` — durable backend wrapping `jj-lib` against a real
   Jujutsu repository under `~/.liquid/workspaces/<id>/`.

Phase 1 needs at least one durable backend so the CLI (`M7`) and FFI bridge
(`M5`) can be exercised end-to-end against real on-disk state. Building a
correct `jj-lib` wrapper, however, is a multi-day engagement on its own:

- `jj-lib` is explicitly API-unstable upstream — every minor version since 0.20
  has broken at least one public surface (`RepoLoader`, `Workspace`,
  `MergedTree`, working-copy materialisation).
- A faithful wrapper requires understanding the operation log, working-copy
  staging, the `MutableRepo` lifecycle, and how to materialise commits onto
  the filesystem without diverging from `jj`'s expectations.
- The Liquid trait (`ContentStore`) is intentionally smaller than jj's surface
  (`Bytes` write of one path per call, no branches, no merges). Mapping every
  trait method onto jj primitives is straightforward in shape but requires
  careful state management.

We need a backend that lands now without committing to a `jj-lib` version we
won't be able to support, and we need a clean upgrade path once we are ready
to do the jj-lib integration properly.

## Decision

Phase 1 ships a `FilesystemContentStore` (TASK-003) that satisfies the
`ContentStore` trait against a flat on-disk layout. The `jj-lib`-backed
`JujutsuContentStore` (TASK-004) is deferred until that integration can be
done properly with a known-good pinned `jj-lib` version.

Both implementations sit behind the same `ContentStore` trait
(per ADR-005), so application code does not change when we swap.

## Rationale

**The trait abstraction is the load-bearing decision, not the engine.** ADR-005
already commits us to interface-first design: storage callers know only the
trait. As long as `FilesystemContentStore` is faithful to the trait's contract
(workspace isolation, atomic writes, op-log replay, exact undo of recorded
operations), the choice of underlying engine is a swap.

**Durability now beats jj-lib later.** The architectural risk we needed to
de-risk in M2 was "does the storage path actually work end-to-end on disk
under our trait" — not "does it use Jujutsu specifically." Shipping a
filesystem backend now lets M3 (auth/permissions) and M5 (FFI) and M7 (CLI)
run against real persistence in days instead of weeks.

**`jj-lib` integration is well-scoped as its own task.** Once a stable jj-lib
version is selected and pinned, the work is mechanical: implement the same
trait against `jj_lib::repo::RepoLoader` + `MutableRepo`, run the existing
integration tests against the new impl. No application code changes.

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Block M3+ on `jj-lib` integration this session | jj-lib's API churn means the integration could take longer than the rest of Phase 1 milestones combined. Sequentially blocking M3–M7 on it is bad scheduling. |
| Use `git2` (libgit2) as a permanent backend | Defeats ADR-001 (the point of jj is the operation log + cleaner conflict model). Switching cost later is the same as `FilesystemContentStore` → `JujutsuContentStore`, with worse Phase-2 properties (no operation log primitive). |
| Skip persistence in Phase 1 entirely; keep only `InMemoryContentStore` | M5/M7 success criteria require an agent's writes to survive process exit ("verify the file is gone" after undo, in §5.2; CLI round-trip in §5.7). In-memory only doesn't satisfy these. |
| Ship a partial `JujutsuContentStore` that compiles but has stub methods | Worst of both: misleads readers into thinking jj is wired up, and adds upstream API debt that gets stale. |

## Consequences

**Easier:**
- M3 (auth + permissions), M5 (FFI), M7 (CLI), and M11 (multi-instance tenant
  config) can all be exercised against real on-disk state immediately.
- Integration tests using `tempfile::TempDir` give us actual durability
  evidence (write → drop store → re-open → read).
- TASK-004 (jj-lib wrapper) gets to work against a known-good test suite —
  the integration tests written for `FilesystemContentStore` apply unchanged
  to `JujutsuContentStore` once it lands.

**Harder:**
- We carry one extra implementation in the codebase until TASK-004 lands. Both
  satisfy the same trait, but they are distinct modules with their own
  bug surface. Mitigation: shared trait-level integration tests run against both.
- Operation log fidelity vs. real Jujutsu: our op log records `Create | Update
  | Delete | Undo` directly; Jujutsu's `op log` is richer (branch creates,
  rebases, etc.). When TASK-004 lands, the trait may need to grow extra
  variants — but only if a caller actually needs them.

**Pinning policy (when TASK-004 lands).**
- Pin `jj-lib` to the exact patch version in `Cargo.lock`. Renovate's existing
  `jj-lib` rule (already in `renovate.json`) blocks auto-upgrade and routes
  bumps to manual review.
- Each `jj-lib` upgrade ships behind its own commit + CI run. The integration
  tests that validate `FilesystemContentStore` are the regression net.
- If `jj-lib` ships a breaking change we can't absorb, fall back to
  `FilesystemContentStore` (it stays in the workspace as the always-available
  backend) until the upgrade is feasible.

**Layout (`FilesystemContentStore`).** Each workspace gets a directory under
the configured root:

```
<root>/<workspace_id>/
  files/<store_path>          # raw bytes; parent dirs auto-created
  op_log.jsonl                # append-only newline-delimited Operation JSON
```

Atomic writes use the standard tmp-then-rename idiom. The op log is parsed by
re-reading the whole file on every `operation_log` / `undo` call — fine for
Phase 1; phase 2+ can add an in-memory cache or a binary log format if it
becomes a hot path.