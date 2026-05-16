# Manual Validation — Phase 1 Milestones M4 → M5

This guide is the auditable companion to the automated test suite for
the **second half of Phase 1's Rust core**:

- **M4** — Cache layer stub (`liquid-cache::ReadCache` +
  `InProcessCache`, wired into `liquid-vcs::CachedContentStore`).
- **M5** — Flutter ↔ Rust FFI bridge (`liquid-sdk-bridge` +
  `flutter_rust_bridge` codegen).

Read it after [`manual-validation-m1-m3.md`](manual-validation-m1-m3.md);
the M1 / M2 / M3 walkthroughs are prerequisites for everything here
(the cache wraps M2's `ContentStore`; the bridge wraps M3's
`PermissionIndex` and M2's store).

## Why a manual guide if `cargo test` already passes?

`cargo test` proves the assertions the authors wrote pass. The
manual walkthrough catches a different class of regression:

- **Caching contract regressions** — does the cache still skip the
  inner `ContentStore` on the second read of the same path, AND does
  a write invalidate the prior bytes before they become visible?
  These are observed by running the M4 walkthrough and inspecting
  the on-disk side-effects + assertions.
- **Tenant-isolation regressions** — does the cache key on
  `(WorkspaceId, StorePath)` everywhere, so two workspaces writing
  to identical paths never bleed through? Cross-workspace bleed-
  through would break Absolute Rule 5 silently.
- **Bridge-surface regressions** (M5) — does every FFI function call
  `require_permission!` BEFORE any other logic (Absolute Rule 4)?
  Does the generated Dart surface match the documented Rust shape?

Run this guide whenever you cut a release tag, merge an M4 / M5
PR, or hand the project off to a new maintainer.

---

## Prerequisites

Same Rust toolchain as the M1-M3 guide:

| Tool | Version | Why |
|---|---|---|
| Rust | `1.94.1` (pinned via `core/rust-toolchain.toml`) | The workspace's only build dependency for the M4 path. |
| `git` | any | To verify branch + commit identity before running. |
| `jq` | optional | Pretty-prints the M2 `op_log.jsonl` re-used in §5.4 cache demos. |

Once M5's Dart side lands, these become required for §M5:

| Tool | Version | Why |
|---|---|---|
| Flutter | stable channel | Drives the M5 Dart integration test (`sdk/liquid_sdk/test/bridge_integration_test.dart`). |
| `flutter_rust_bridge_codegen` | matches the version pinned in `sdk/liquid_sdk/pubspec.yaml` | Regenerates `app/lib/bridge/*` so a contributor can confirm the committed bindings match the Rust surface byte-for-byte. |

The M5 § carries a top-of-section `STATUS — PENDING` block so the
guide can be reviewed end-to-end today and re-read unchanged when
M5 merges (and the STATUS block flips to `DONE`).

```sh
cd <repo-root>
rustc --version           # should print 1.94.1
git rev-parse HEAD        # record for the run-log; sign-off bundle
```

---

## M4 — Cache layer stub (`liquid-cache` + `CachedContentStore`)

**Spec:** `IMPLEMENTATION_PLAN.md §5.4`, success criterion: "Second
read of the same content hits the cache."

**What you are validating:** the `ReadCache` trait + the
`InProcessCache` backend (M4-side), and the `CachedContentStore<S,
C>` wrapper that wires them into any `ContentStore` (the M2
filesystem store, in this walkthrough). The wrapper must:

1. Serve the second read of the same `(workspace, path)` from the
   cache without touching the inner store.
2. Invalidate the prior content hash BEFORE a successful write
   becomes visible, so the next read observes the new bytes.
3. Conservatively invalidate every cached entry for a workspace on
   `undo`, so a subsequent read falls through and re-warms
   (Phase-1 limitation per §5.4: precise per-path invalidation
   waits on TASK-004's `jj-lib` backend).
4. Pass through `list` and `operation_log` unchanged.
5. Key the index on `(WorkspaceId, StorePath)`, so a write under
   one workspace never affects the cache entry of another
   workspace's identical path (Absolute Rule 5).

### Step M4.1 — Focused tests

```sh
cargo test -p liquid-cache --manifest-path core/Cargo.toml \
  2>&1 | .claude/hooks/filter-test-output.sh
cargo test -p liquid-vcs --manifest-path core/Cargo.toml --test cached_store \
  2>&1 | .claude/hooks/filter-test-output.sh
```

**Expected:** two test-result summary lines:

- `liquid-cache`: **8 passed; 0 failed** — covers `InProcessCache`
  put/get/invalidate round-trips, content-addressable dedup across
  paths, and missing-key invalidation being a no-op.
- `liquid-vcs::cached_store`: **8 passed; 0 failed** — covers the
  wrapper. The plan-level success-criterion test is named
  `second_read_of_same_path_is_served_from_cache` and uses a
  `SpyStore` that counts every inner-store call: the assertion
  fails if the second read reaches the inner store.

**Regression shape:** if `second_read_of_same_path_is_served_from_cache`
fails with `SpyStore::counts().read == 2`, the wrapper has stopped
warming the index or the cache has stopped honouring `put`. If
`write_invalidates_prior_hash_so_next_read_observes_new_content`
fails, the write path is leaving a stale entry behind — a silent
correctness bug far worse than a perf regression.

### Step M4.2 — Walkthrough example (recommended)

```sh
cargo run --manifest-path core/Cargo.toml -p liquid-vcs \
  --example m4_walkthrough
```

**Expected:** the example runs four asserted phases against a real
`FilesystemContentStore` under `/tmp/liquid-m4-walkthrough/` (plus
a one-line setup banner for the workspace + author), prints a
line per phase, and exits 0:

```
M4 walkthrough — Cache layer wired into FilesystemContentStore
  root: /tmp/liquid-m4-walkthrough
  workspace: <uuid>
  author:    user:<uuid>
  write  pages/welcome.md       -> commit <uuid>
  read   pages/welcome.md  x2  -> 25 bytes (second served from cache)
  write  pages/welcome.md       -> commit <uuid> (overwrite)
  read   pages/welcome.md       -> observes new bytes (no stale hit)
  tenancy: ws-a/pages/shared.md != ws-b/pages/shared.md (cache keyed on workspace)
  undo   op <uuid> -> synthetic commit <uuid>
  read   pages/welcome.md       -> 25 bytes (Update undone; cache re-warmed from inner)

M4 walkthrough OK
Inspect the on-disk state: ls -la /tmp/liquid-m4-walkthrough/<uuid>
```

Every step uses `assert!()` / `assert_eq!()`, so a panic === broken
M4 contract. The walkthrough lives at
[`core/liquid-vcs/examples/m4_walkthrough.rs`](../core/liquid-vcs/examples/m4_walkthrough.rs).
After the run, two artifacts on disk make the demo auditable:

```sh
ls -la /tmp/liquid-m4-walkthrough/        # one dir per workspace UUID
cat /tmp/liquid-m4-walkthrough/<uuid>/op_log.jsonl   # the underlying op log
```

The wrapper leaves no on-disk cache state (`InProcessCache` is
in-memory only — Phase 1, §9); the artifacts you can `ls` are
the inner `FilesystemContentStore`'s normal layout. A regression in
the cache layer manifests as assertion failures in the example
output, not as stale files on disk.

Clean up between runs with:

```sh
just clean-walkthroughs   # removes /tmp/liquid-m*-walkthrough
```

### Step M4.3 — Cache invariants by inspection

Read the wrapper one file at a time and confirm each guarantee.

```sh
grep -n 'fn read\|fn write\|fn undo\|fn list\|fn operation_log' \
  core/liquid-vcs/src/cached.rs
```

Confirm by eye:

1. **`read`** at `core/liquid-vcs/src/cached.rs:95` — checks the
   `(workspace, path) → hash` index, asks the cache for bytes on
   hit, and only falls through to the inner store on miss or
   stale-index. Errors (e.g. `NotFound`) must NOT be cached; the
   wrapper achieves this by hashing only AFTER `inner.read` returns
   `Ok(bytes)`.
2. **`write`** at `:117` — removes the index entry FIRST, invalidates
   the cache entry, THEN delegates to the inner store. The known
   Phase-1 limitation is documented in the doc-comment: a failing
   inner write leaves the cache invalidated even though the
   inner store still holds the old bytes; the wrapper trades a
   silent perf regression on failure for correctness (the next
   read re-hashes and re-warms).
3. **`undo`** at `:150` — drains every index entry for the affected
   workspace and invalidates each in the cache; the comment cites
   `IMPLEMENTATION_PLAN.md §5.4` and the TASK-004 follow-up.
4. **`list`** at `:172` and **`operation_log`** at `:146` — pass
   through verbatim. The cache is content-keyed; listing a prefix
   does not involve content hashes, so caching it would only
   introduce its own invalidation problem.

**Regression shape:** any new `unwrap()` / `expect()` in
`core/liquid-vcs/src/cached.rs` outside `#[cfg(test)]` violates
Absolute Rule 1 — even the `Mutex` lock recovers from poison via
`unwrap_or_else(PoisonError::into_inner)`. Check with:

```sh
grep -nE 'unwrap\(\)|expect\(' core/liquid-vcs/src/cached.rs \
  | grep -vE 'cfg.*test'
```

The output must be empty.

### Step M4.4 — Lints

```sh
cargo clippy --manifest-path core/Cargo.toml -p liquid-cache -p liquid-vcs \
  --all-targets --locked -- -D warnings \
  2>&1 | .claude/hooks/filter-test-output.sh
```

**Expected:** no warnings. The workspace lints in `core/Cargo.toml`
deny `unsafe_code` and warn on `unwrap_used` / `expect_used` /
`panic` outside `#[cfg(test)]`.

---

## M5 — Flutter ↔ Rust FFI bridge (`liquid-sdk-bridge`)

**Spec:** `IMPLEMENTATION_PLAN.md §5.5`, success criterion: "Dart
test creates a workspace, writes a page, reads it back, and the
round-trip data matches."

**STATUS — PENDING.** As of the most recent merge to `main`, M5 is
NOT yet implemented. The placeholder crate is in place
(`core/liquid-sdk-bridge/`) but its `src/lib.rs` is empty other
than a doc-comment pointing here. Phase-1 sequencing reasons:

- M5 wraps the Dart-side `flutter_rust_bridge` codegen output,
  which lives under `app/lib/bridge/` and `sdk/liquid_sdk/`.
  Neither directory has been scaffolded yet — they land with M6 /
  M6.5 (`IMPLEMENTATION_PLAN.md §5.6 + §5.7`).
- The CLI-before-UI rule (Absolute Rule 6) means M6.5 (minimal
  agent CLI) lands before M5's Dart integration test runs; M5
  itself can land standalone but its happy-path proof needs a
  Flutter test runner.

The section below is the **review checklist** an auditor follows
when M5 lands, plus the manual steps a reviewer runs to validate
the surface. Treat it as the specification for the eventual M5
PR — every step has a concrete pass condition.

The M5 PR will assign a TASK-NNN id and update this section in the
same commit; until then, "TASK-pending" is the placeholder.

### Step M5.0 — Confirm scaffold + dependency wiring

```sh
ls core/liquid-sdk-bridge/src/                           # must include lib.rs + the new module files
grep -n 'flutter_rust_bridge' core/liquid-sdk-bridge/Cargo.toml
grep -rn '#\[frb' core/liquid-sdk-bridge/src/ | head
```

**Pass:** `lib.rs` re-exports the five M5 functions; `Cargo.toml`
adds `flutter_rust_bridge` with the same major version that the
Dart-side `pubspec.yaml` depends on; every `pub async fn` in the
crate carries an `#[frb(sync)]` or `#[frb(stream)]` attribute as
appropriate.

### Step M5.1 — Five FFI functions present

```sh
grep -nE 'pub async fn (create_workspace|list_workspaces|load_page|write_page|check_permission)' \
  core/liquid-sdk-bridge/src/
```

**Pass:** every name from `IMPLEMENTATION_PLAN.md §5.5`'s code block
appears exactly once with the documented arity:

```rust
pub async fn create_workspace(name: String) -> Result<WorkspaceId>;
pub async fn list_workspaces(principal: String) -> Result<Vec<WorkspaceSummary>>;
pub async fn load_page(workspace: WorkspaceId, page_id: PageId) -> Result<PageSnapshot>;
pub async fn write_page(workspace: WorkspaceId, page_id: PageId,
                        snapshot: PageSnapshot, author: String,
                        message: String) -> Result<CommitId>;
pub async fn check_permission(principal: String, action: String,
                              resource: String) -> Result<bool>;
```

### Step M5.2 — Permission check is the FIRST line of every function

```sh
grep -nE 'pub async fn .*\{' core/liquid-sdk-bridge/src/*.rs
grep -nA1 'pub async fn ' core/liquid-sdk-bridge/src/*.rs \
  | grep -A1 'pub async fn' | grep -B1 'require_permission!'
```

**Pass:** every function body starts with `require_permission!(...)` — no
other logic, no logging, no validation runs first. This is the
spec-level enforcement of `CLAUDE.md`'s Absolute Rule 4. The
linter will not catch a permission check that runs second or
third; the reviewer must verify by reading the first executable
line of every function.

**Regression shape:** if `require_permission!` is wrapped in any
control flow (e.g. `if let Some(...) = ...`) before being called,
the rule is broken. Reject the PR with a citation back to
Absolute Rule 4.

### Step M5.3 — Codegen output matches the Rust surface

`flutter_rust_bridge` writes generated Dart bindings into
`app/lib/bridge/`. The PR must commit these files (they are
machine-generated but version-controlled — see §5.5: "generated
files must not be manually edited"). Confirm:

```sh
cd <repo-root>
flutter_rust_bridge_codegen generate --no-write
diff -r app/lib/bridge/ /tmp/rerun-bridge-output/
```

**Pass:** the regenerated output is byte-identical to the
committed files. Any diff means the contributor edited generated
files by hand, or the codegen version drifted from the version
pinned in `sdk/liquid_sdk/pubspec.yaml`. Either is a hard rejection.

### Step M5.4 — Dart integration test round-trip

```sh
cd sdk/liquid_sdk
flutter test test/bridge_integration_test.dart
```

**Pass:** the test creates a workspace, provisions a principal,
writes a page, reads it back, and asserts the round-trip data is
byte-identical. The plan-level success criterion in §5.5 is this
exact test. Output should end with `+1: All tests passed!`.

**Regression shape:** if the test fails with a serde mismatch on
`PageSnapshot` or `WorkspaceSummary`, the Rust-side types changed
without a matching codegen run. Re-run §M5.3 first.

### Step M5.5 — End-to-end via the M3 + M4 stack

The bridge wraps three previous milestones. A manual sanity-check
that exercises the whole stack:

```sh
cargo test -p liquid-sdk-bridge --manifest-path core/Cargo.toml \
  2>&1 | .claude/hooks/filter-test-output.sh
```

**Pass:** the crate's own Rust-side unit tests (each FFI function
in isolation, mocking the inner services where useful) pass; the
suite must include at least one test per public function asserting
the permission-denied path returns `Err(LiquidError::Forbidden)`
without touching the inner store.

### Step M5.6 — Lints

```sh
cargo clippy --manifest-path core/Cargo.toml -p liquid-sdk-bridge \
  --all-targets --locked -- -D warnings \
  2>&1 | .claude/hooks/filter-test-output.sh
```

**Pass:** no warnings. The crate carries the same workspace lint
config as everything else.

---

## Sign-off checklist

Tick every box before stamping the run-log:

- [ ] M4 — Step M4.1 + M4.2 + M4.3 + M4.4 all green; the
      walkthrough exits 0 and prints the documented matrix lines.
- [ ] M4 — `grep -nE 'unwrap\(\)|expect\(' core/liquid-cache/src/
            core/liquid-vcs/src/cached.rs | grep -vE 'cfg.*test'`
      is empty.
- [ ] M5 — STATUS still PENDING ⇒ open the M5 PR follow-up issue
      with a link back to this guide's §M5 review checklist; do
      NOT tag Phase-1 complete.
- [ ] M5 — when STATUS flips to DONE: every M5.x step green,
      generated bridge files byte-identical to a fresh codegen
      run, Dart integration test reports `+1: All tests passed!`.
- [ ] Cross-milestone — `cargo test --workspace --locked` green;
      `cargo clippy --workspace --all-targets --locked -- -D
      warnings` clean; `cargo fmt --all --check` clean.
- [ ] `just deny-check` clean (advisories, licenses, bans, sources
      all ok).
- [ ] `just coverage-check` clean (≥80% workspace coverage gate).

If any line above is unchecked, the milestone is **not** done; do
not tag the release.

---

## Related documents

- [`manual-validation-m1-m3.md`](manual-validation-m1-m3.md) — the
  predecessor guide that this one assumes as background.
- `IMPLEMENTATION_PLAN.md` §4 (interfaces), §5.4 / §5.5 (M4 / M5
  plan), §9 (per-crate reference) — the authoritative spec.
- `core/liquid-vcs/examples/m4_walkthrough.rs` — the runnable M4
  artifact this guide drives.
- `core/liquid-vcs/tests/cached_store.rs` — the M4 SpyStore-based
  success-criterion test (`second_read_of_same_path_is_served_from_cache`).
- `docs/adr/001-jujutsu-pinning.md` — why the cache's
  `undo`-invalidation is workspace-wide today (precise
  invalidation waits on the `jj-lib` backend).
- `CHANGELOG.md` — every M4 / M5 surface change ships with a
  matching `## [Unreleased]` entry.
