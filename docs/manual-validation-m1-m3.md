# Manual Validation — Phase 1 Milestones M1 → M3

This guide walks a human reviewer through validating the **entire**
Phase-1 core layer (workspace primitives → VCS layer → auth +
permissions) **manually**, using only the toolchain pinned in
`core/rust-toolchain.toml` and the runnable artifacts that ship in
the workspace.

It is the auditable companion to the automated test suite. Read it
top-to-bottom; every step gives you a concrete command, the
expected outcome, and what a regression would look like.

## Why a manual guide if `cargo test` already passes?

`cargo test` proves the assertions the authors wrote pass. The
manual walkthrough catches a different class of regression:

- **API ergonomics regressions** — does the public surface look the
  way the spec promises? (`IMPLEMENTATION_PLAN.md §4` is the
  contract.) The walkthrough examples use only the public surface
  and have to compile against the published types.
- **On-disk format regressions** — is the layout under
  `<root>/workspaces/<id>/...` still what `ADR-001` and §9
  describe? The walkthroughs keep their artifacts behind so you can
  `ls` and `cat` them.
- **Cross-crate wiring regressions** — does the
  `issue_token → validate_token → require_permission!` chain still
  work end-to-end, even when each crate ships in isolation? The
  M3 walkthrough is the spec's plan-level success criterion turned
  into a runnable artifact.

Run this whenever you cut a release tag, merge a milestone PR, or
hand the project off to a new maintainer.

---

## Prerequisites

| Tool | Version | Why |
|---|---|---|
| Rust | `1.94.1` (pinned via `core/rust-toolchain.toml`) | The workspace's only build dependency. |
| `git` | any | To verify branch + commit identity before running. |
| `jq` | optional | Pretty-prints the M2 `op_log.jsonl`. |

You do **not** need `just`, `bats`, `lefthook`, `flutter`, or
`cargo-deny` for this walkthrough — those gate CI; the Phase-1
manual validation only touches Rust.

```sh
cd <repo-root>
rustc --version           # should print 1.94.1
git rev-parse HEAD        # record for the run-log; sign-off bundle
```

If `rustc --version` does not print `1.94.1`, install Rust via
<https://rustup.rs> and let the `rust-toolchain.toml` pin drive the
auto-install.

---

## M1 — Workspace bootstrap + `liquid-core` primitives

**Spec:** `IMPLEMENTATION_PLAN.md §5.1` (success criterion: 90%
coverage on `liquid-core`).

**What you are validating:** every ID type
(`WorkspaceId`, `AppInstanceId`, `ComponentId`, `PageId`,
`PrincipalId`, `OperationId`, `CommitId`, `ContentHash`),
`StorePath` (with `..`/empty/absolute rejection), `SlotName`,
`SlotValue`, the `Action` / `Resource` enums, the `TenantConfig`,
and the workspace-wide `LiquidError`.

### Step M1.1 — Focused tests

```sh
cargo test -p liquid-core --manifest-path core/Cargo.toml \
  2>&1 | .claude/hooks/filter-test-output.sh
```

**Expected:** a single test-result summary line — `26 passed; 0
failed; 0 ignored` (plus the trailing `0 passed` line for doctests
when none exist). The filter hook captures the raw log under
`.ai/artifacts/logs/raw-<ts>.log` and prints a compact summary;
the suite covers ID construction, equality, serde round-trips,
`StorePath` rejection of `..` / absolute / empty paths, and
`ContentHash` hex-length / lowercase validation.

**Regression shape:** any "FAILED" line means a primitive contract
moved. The downstream M2 / M3 walkthroughs will then fail to
compile or panic — fix M1 first, then re-run the rest.

### Step M1.2 — API surface tour

The public surface lives in `core/liquid-core/src/lib.rs`. Open it
and confirm — by eye — that every name in the M1 §5.1 sub-list is
re-exported. Names absent from `lib.rs` are not visible to other
crates and the M2 / M3 walkthroughs would not compile.

```sh
grep '^pub' core/liquid-core/src/lib.rs
```

**Expected** (mod statements + re-exports; ordering may differ):

```
pub mod content_hash;
pub mod error;
pub mod ids;
pub mod permission;
pub mod slot;
pub mod store_path;
pub mod tenant;
pub use content_hash::ContentHash;
pub use error::{LiquidError, Result};
pub use ids::{
    AppInstanceId, CommitId, ComponentId, OperationId, PageId, PrincipalId, RoleId, WorkspaceId,
};
pub use permission::{Action, Resource};
pub use slot::{SlotName, SlotValue};
pub use store_path::StorePath;
pub use tenant::TenantConfig;
```

**Regression shape:** a missing re-export means downstream crates
have to qualify with the inner-module path (e.g.
`liquid_core::ids::AppInstanceId`), and the M2 / M3 walkthroughs
break. A new ID type added to `ids.rs` without being re-exported
is invisible to downstream crates.

### Step M1.3 — Lints (`unsafe_code` + `unwrap`)

```sh
cargo clippy --manifest-path core/Cargo.toml -p liquid-core \
  --all-targets --locked -- -D warnings \
  2>&1 | .claude/hooks/filter-test-output.sh
```

**Expected:** no warnings. The workspace lints in `core/Cargo.toml`
deny `unsafe_code` and warn on `unwrap_used` / `expect_used` /
`panic` outside `#[cfg(test)]`; a warning here is an Absolute Rule
violation per `CLAUDE.md`.

---

## M2 — VCS layer (`liquid-vcs`)

**Spec:** `IMPLEMENTATION_PLAN.md §5.2`, `ADR-001` (filesystem
backend; `jj-lib` deferred to TASK-004).

**What you are validating:** the `ContentStore` trait + both
shipped backends (`InMemoryContentStore`, `FilesystemContentStore`)
satisfy a workspace-create → write-three → read-back → undo → not-found
cycle, and the on-disk layout matches `<root>/<workspace_id>/files/`
+ `op_log.jsonl` per ADR-001.

### Step M2.1 — Focused tests

```sh
cargo test -p liquid-vcs --manifest-path core/Cargo.toml \
  2>&1 | .claude/hooks/filter-test-output.sh
```

**Expected:** two test-result summary lines totalling 26 passed
across the two suites — `14 passed` (filesystem integration tests
in `core/liquid-vcs/tests/filesystem_store.rs`) and `12 passed`
(in-memory unit tests in `core/liquid-vcs/src/in_memory.rs`).
Doc-tests print an additional `0 passed`. Together the suites
cover the M2 plan-level success criterion (create → write three →
read back → undo → NotFound) against both backends, plus the
durability test that re-opens the same `FilesystemContentStore`
root in a fresh struct and reads the data back.

### Step M2.2 — Walkthrough example (recommended)

A self-asserting runnable demonstration ships at
`core/liquid-vcs/examples/m2_walkthrough.rs`. It reproduces the M2
success criterion against `FilesystemContentStore`, with the
artifacts left under your system temp dir for inspection.

```sh
cargo run --manifest-path core/Cargo.toml -p liquid-vcs \
  --example m2_walkthrough
```

**Expected** (uuid values will differ):

```
M2 walkthrough — Filesystem ContentStore
  root: /tmp/liquid-m2-walkthrough
  workspace: <uuid>
  author: user:<uuid>
  write  pages/welcome.md     -> commit <uuid>
  write  pages/notes.md       -> commit <uuid>
  write  pages/todo.md        -> commit <uuid>
  read   pages/welcome.md     -> 37 bytes (OK)
  read   pages/notes.md       -> 23 bytes (OK)
  read   pages/todo.md        -> 46 bytes (OK)
  list   pages/notes.md
  list   pages/todo.md
  list   pages/welcome.md
  op-log size=3 newest_op=<uuid> newest_path=pages/todo.md
  undo   op <uuid> -> synthetic commit <uuid>
  read   pages/todo.md       -> NotFound (as expected)
  layout files_dir=… op_log lines=4

M2 walkthrough OK
Inspect the on-disk state: ls -la <root>/<workspace> && cat <root>/<workspace>/op_log.jsonl
```

Exit code 0 ⇒ M2 satisfies its plan-level success criterion. Any
panic ⇒ a regression in the shipped behaviour; the message
identifies which assertion failed.

### Step M2.3 — Inspect the on-disk layout (ADR-001)

The walkthrough ends with the exact commands to run; reproduce
them here so the layout is visible to a human reviewer:

```sh
WS_ROOT=$(ls -d /tmp/liquid-m2-walkthrough/*/)
echo "workspace root: $WS_ROOT"

ls -la "$WS_ROOT"
# Expected entries:
#   files/                 — directory of raw bytes, one per StorePath
#   op_log.jsonl           — newline-delimited Operation JSON

ls -la "$WS_ROOT/files/pages/"
# Expected entries:
#   welcome.md   notes.md   (todo.md was undone — must be absent)

cat "$WS_ROOT/op_log.jsonl"
# Expected: 4 JSONL records — three Create, one Undo.
# Pipe through `jq -c .` to validate the shape.

cat "$WS_ROOT/op_log.jsonl" | jq -c 'keys' | sort -u
# Expected: a single combined key set per Operation —
#   ["author","commit","id","kind","message","timestamp_unix_millis"]
```

**Regression shape:**

- `files/` missing ⇒ atomic-write idiom regressed.
- `op_log.jsonl` not newline-delimited or unparseable ⇒ the audit
  log format changed; M6.5's `audit list` command and any
  M3-permission audit will break.
- `pages/todo.md` still present ⇒ `undo` no longer inverts the
  most recent write.

### Step M2.4 — Cleanup

```sh
rm -rf /tmp/liquid-m2-walkthrough
```

The example wipes the root at the start of every run, so cleanup
is only required if you want a clean machine.

---

## M3 — Auth + permissions (`liquid-auth` + `liquid-permissions`)

**Spec:** `IMPLEMENTATION_PLAN.md §5.3`, `ADR-002` (trait scoping
decisions). Success criterion: an end-to-end test that wires the
two crates along the bridge-call shape (`issue_token → validate
token → require_permission!`) and proves the
`AppViewer` / `AppEditor` / `WorkspaceOwner` matrix.

**What you are validating:** the `LocalIdentityProvider`
(Argon2id-hashed passwords + HMAC-SHA256 session tokens), the
`InMemoryPermissionIndex` + `FilesystemPermissionIndex`
implementations of `PermissionIndex`, the `require_permission!`
macro, and that all auth failure modes collapse to
`LiquidError::Forbidden` (no mode-leak).

### Step M3.1 — Focused tests

```sh
cargo test -p liquid-auth -p liquid-permissions \
  --manifest-path core/Cargo.toml \
  2>&1 | .claude/hooks/filter-test-output.sh
```

**Expected** (counts may grow as new tests land — never shrink):

| Crate | Suite | Pass count |
|---|---|---|
| `liquid-auth` | `local_provider` integration | 13 |
| `liquid-auth` | `local_provider_corners` | 5 |
| `liquid-permissions` | `permission_index` unit | 14 |
| `liquid-permissions` | `filesystem_index` integration | 9 |
| `liquid-permissions` | `filesystem_corners` | 4 |
| `liquid-permissions` | `m3_end_to_end` (success criterion) | 1 |

The `m3_end_to_end::m3_app_viewer_cannot_write_app_editor_can_owner_can_both`
test is the plan-level success criterion. If it fails, M3 is not
shipped.

### Step M3.2 — Walkthrough example (recommended)

A self-asserting runnable demonstration ships at
`core/liquid-permissions/examples/m3_walkthrough.rs`. It exercises
the bridge-call shape against **both** the in-memory and the
filesystem permission indexes, and proves the token-validation
negative surface (tampered / wrong-key / malformed all collapse to
`Forbidden`).

```sh
cargo run --manifest-path core/Cargo.toml -p liquid-permissions \
  --example m3_walkthrough
```

**Expected** (uuid values will differ):

```
M3 walkthrough — auth + permissions
  root: /tmp/liquid-m3-walkthrough
  workspace: <uuid>
  app:       <uuid>
  register user alice -> user:<uuid>
  provision viewer-bot -> agent:<uuid>
  provision editor-bot -> agent:<uuid>
  token format: <principal>.<expires_unix>.<hmac_hex> — round-trip ok
  --- InMemoryPermissionIndex ---
  in-memory matrix: viewer write=Forbidden  editor write=OK  owner read+write=OK  viewer read=OK
  --- FilesystemPermissionIndex (durable) ---
  fs matrix after reopen: viewer write=Forbidden  editor write=OK  owner write=OK
  token negatives: tampered=Forbidden  wrong-key=Forbidden  malformed=Forbidden  expired=Forbidden

M3 walkthrough OK
Inspect the on-disk state:
  cat /tmp/liquid-m3-walkthrough/auth/users.toml
  cat /tmp/liquid-m3-walkthrough/auth/agents.toml
  cat /tmp/liquid-m3-walkthrough/perm/workspaces/<uuid>/permissions.toml
```

Exit code 0 ⇒ M3 satisfies the §5.3 success criterion **and** the
disk-persistence acceptance from TASK-007.

### Step M3.3 — Inspect the on-disk format

```sh
# Argon2id-hashed user credentials (§5.3 + §9 liquid-auth layout).
cat /tmp/liquid-m3-walkthrough/auth/users.toml
# Expected structure (one entry per registered user):
#   [[users]]
#   id            = "<uuid>"
#   username      = "alice"
#   password_hash = "$argon2id$v=19$m=...$..."
# Verify password_hash starts with "$argon2id$"; ANY plaintext
# password is a security regression.

cat /tmp/liquid-m3-walkthrough/auth/agents.toml
# Expected structure:
#   [[agents]]
#   id            = "<uuid>"
#   name          = "viewer-bot"
#   workspace_id  = "<workspace-uuid>"
#   authorized_by = "user:<owner-uuid>"      # principal_to_string
#   created_unix  = <unix seconds>
# Verify every agent records who authorised it; missing
# `authorized_by` = audit hole. The `user:` / `agent:` prefix in
# `authorized_by` is mandatory — a bare UUID indicates a
# regression in `principal_to_string`.

PERM_FILE=$(ls /tmp/liquid-m3-walkthrough/perm/workspaces/*/permissions.toml)
cat "$PERM_FILE"
# Expected structure (per TASK-007 + §9). PrincipalId and Resource
# are adjacently-tagged enums, so they serialise as TOML inline
# tables, not strings:
#
#   [[bindings]]
#   role = "workspace_owner"                # serde rename_all = "snake_case"
#
#   [bindings.principal]
#   kind = "user"                           # or "agent"
#   id   = "<uuid>"
#
#   [[bindings]]
#   role = "app_viewer"                     # or "app_editor", "agent", "workspace_member"
#
#   [bindings.principal]
#   kind = "agent"
#   id   = "<uuid>"
#
#   [bindings.scope]                        # omitted for workspace-wide roles
#   kind = "app_instance"                   # or "workspace", "component", "page", "field"
#   id   = "<uuid>"
```

**Regression shape:**

- Any user with a raw password instead of an Argon2id hash ⇒
  release-blocker. Discard the build.
- An agent file without `authorized_by` (or with a bare-UUID
  `authorized_by`) ⇒ provision-trail regression; audits become
  unreliable.
- A `permissions.toml` whose `role` string is not in the
  `BuiltInRole` enum (snake-case form) ⇒
  `FilesystemPermissionIndex::open` will return
  `InvalidInput` on next start; the disk format has drifted from
  the enum.
- `principal` or `scope` serialised as a bare string ⇒ adjacent-tag
  serde policy regressed; on-disk format is no longer round-trip
  compatible with `PrincipalId` / `Resource`.

### Step M3.4 — Token negative surface (§4.5 no-mode-leak)

The session token format is `principal . expires_unix . hmac_hex`
and every failure mode must collapse to `LiquidError::Forbidden`
(§4.5). The walkthrough already proves the four families
(tampered, wrong-key, malformed) — to double-check at the focused
test level:

```sh
cargo test -p liquid-auth --manifest-path core/Cargo.toml \
  --test local_provider validate_token_ \
  -- --nocapture 2>&1 | tail -20
```

**Expected:** four tests, all `ok`:

- `validate_token_rejects_tampered_token`
- `validate_token_rejects_token_signed_with_wrong_secret`
- `validate_token_rejects_expired_token`
- `validate_token_rejects_malformed_token`

A failure here means a token rejection path is leaking which mode
failed — a confidentiality regression.

### Step M3.5 — Cleanup

```sh
rm -rf /tmp/liquid-m3-walkthrough
```

---

## Cross-milestone integration

The Phase-1 core layer is M1 + M2 + M3. The walkthrough examples
prove each in isolation; the **integration** proof is the
`m3_end_to_end` test, which already uses `liquid-core` primitives
(M1), the in-memory permission index (M3), and the local
identity provider (M3) along the exact path the bridge will
follow:

```sh
cargo test -p liquid-permissions --manifest-path core/Cargo.toml \
  --test m3_end_to_end -- --nocapture 2>&1 | tail -10
```

**Expected:** one test, `ok`. Failure here is a release-blocker
for Phase 1.

The M2 layer is not part of the M3 end-to-end test because the
bridge wires `ContentStore` to permissions via
`liquid-sdk-bridge` (M5), which has not yet shipped. The
walkthrough examples are the closest manual proxy.

---

## Sign-off checklist

Before tagging a Phase-1-M3 release or handing the project off:

- [ ] `rustc --version` prints `1.94.1`.
- [ ] `git rev-parse HEAD` is recorded in the sign-off bundle.
- [ ] M1 — Step M1.1 + M1.2 + M1.3 all green.
- [ ] M2 — Step M2.1 + M2.2 + M2.3 all green; on-disk layout
      matches ADR-001 exactly.
- [ ] M3 — Step M3.1 + M3.2 + M3.3 + M3.4 all green;
      `password_hash` starts with `$argon2id$`; every agent record
      has `authorized_by` with the `user:` / `agent:` prefix.
- [ ] Cross-milestone — `m3_end_to_end` test green.
- [ ] `cargo clippy --workspace --all-targets --locked -- -D
      warnings` clean.
- [ ] `cargo fmt --all --check` clean.
- [ ] `just deny-check` clean (advisories, licenses, bans,
      sources all ok).

If any line above is unchecked, the milestone is **not** done; do
not tag the release.

---

## Related documents

- `IMPLEMENTATION_PLAN.md` §4 (interfaces), §5 (Phase-1 plan), §9
  (per-crate reference) — the authoritative spec.
- `docs/adr/001-jujutsu-pinning.md` — why M2 ships a filesystem
  backend and defers `jj-lib`.
- `docs/adr/002-m3-trait-scoping.md` — why §4.2 / §4.5 dropped
  `grant`, `RoleId`, and the workspace-bound token field.
- `docs/security/threat-model.md` — the threat model the M3
  auth + permissions surface is built against.
- `docs/ops/branch-protection.md` — the GitHub-side enforcement
  the maintainer adds before tagging a release.
- `CHANGELOG.md` — every M1 / M2 / M3 surface change ships with a
  matching `## [Unreleased]` entry.
