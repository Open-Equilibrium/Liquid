# Manual Validation — Phase 1 Milestone M6.5 (Minimal Agent CLI)

This guide is the auditable companion to `bats tests/cli/` for the
**Phase-1 agent CLI**: the seven subcommands in
`IMPLEMENTATION_PLAN.md §5.6` (TASK-008) that drive the MVP
data path end-to-end.

Read it after [`manual-validation-m1-m3.md`](manual-validation-m1-m3.md)
and [`manual-validation-m4-m5.md`](manual-validation-m4-m5.md) —
M6.5 composes every Phase-1 backend (`liquid-auth` +
`liquid-permissions` + `liquid-vcs` + `liquid-sdk-bridge`) under a
single `liquid` binary.

## Why a manual guide if `bats tests/cli/` already passes?

`bats` proves the asserted behaviours pass. The manual walkthrough
catches a different class of regression:

- **State-layout drift** — does `$LIQUID_HOME` still look like §5.6
  documents (`auth/`, `vcs/`, `perm/`, `registry/`, `secret`,
  `token`)? The walkthrough keeps the dir behind so you can
  `ls -la`.
- **Output-shape drift** — does `--format json` still emit the
  envelope `{ "ok", "data", "records", "error" }` the agent
  harness expects? Eyes-on grep of one line per subcommand.
- **Permission-model drift** — does an `AppViewer`-bound agent
  really get `Forbidden` on `page write`, not `NotFound`?
  Manually walking the matrix per role catches a regression that
  `bats` could miss if its negative test were itself a placebo.
- **Cross-process persistence** — does a second `liquid` invocation
  see the workspaces a prior one created? The CLI is short-lived;
  every command re-opens the four backends.

Run this whenever you cut a release tag, merge the M6.5 PR, or
hand the project off to a new maintainer.

---

## Prerequisites

| Tool | Version | Why |
|---|---|---|
| Rust | `1.94.1` (pinned via `core/rust-toolchain.toml`) | Build the `liquid` binary. |
| `bats` | latest | Run the regression suites. |
| `jq` | any | Parse the CLI's JSON envelope in the walkthrough. |

```sh
cd <repo-root>
cargo build --manifest-path core/Cargo.toml -p liquid-cli
export PATH="$PWD/core/target/debug:$PATH"
liquid --version       # confirms the binary is on PATH
```

If you skip the `PATH` export, the bats suites will fall back to
`core/target/debug/liquid` automatically (per the `setup()` hook
in `tests/cli/00_mvp_slice.bats` and `10_cli_subcommands.bats`).

---

## M6.5 — Minimal agent CLI (`liquid-cli`)

**Spec:** `IMPLEMENTATION_PLAN.md §5.6` + `§12 Agent CLI
Specification`. Success criterion: `bats tests/cli/00_mvp_slice.bats`
passes end-to-end + the AppViewer-cannot-write negative path
proves Absolute Rule 4.

**What you are validating:**

- The seven §5.6 subcommands exist and produce the documented
  envelope shapes.
- `$LIQUID_HOME` is fully self-contained — a fresh `mktemp -d`
  bootstraps cleanly.
- Every command validates its token first (collapsing all auth
  failures to `Forbidden` per §4.5); every write/undo runs
  `require_permission!` before any state mutation (Absolute Rule
  4).
- The on-disk state survives across independent `liquid`
  invocations.

### Step M6.5.1 — Focused tests

```sh
cargo test --manifest-path core/Cargo.toml -p liquid-sdk-bridge \
  2>&1 | .claude/hooks/filter-test-output.sh
bats tests/cli/00_mvp_slice.bats tests/cli/10_cli_subcommands.bats \
  2>&1 | .claude/hooks/filter-test-output.sh
```

**Expected:** the bridge tests are at parity with §M5; the bats
suites report:

- `tests/cli/00_mvp_slice.bats`: **6 passed; 0 failed** — every
  step of the MVP happy path plus the AppViewer-cannot-write
  negative path.
- `tests/cli/10_cli_subcommands.bats`: **16 passed; 0 failed** —
  per-subcommand focused coverage: `--version`, no-args help-exit,
  bootstrap files, registry cross-process persistence, `auth
  token` happy + no-token, invalid workspace UUID, mutually
  exclusive `--data` / `--file` plus `--file` body source,
  NotFound on unknown read, audit `--action Write` filter, audit
  `--principal a:<uuid>` short-form filter, audit `--action Undo`
  discriminates from `Write`, bootstrap edge-case (user exists
  but token file missing) surfaces actionable error,
  text-format summary, text-format stderr error on bad UUID.

### Step M6.5.2 — Manual walkthrough

Pick a fresh state root so the demo cannot collide with anything:

```sh
export LIQUID_HOME="$(mktemp -d -t liquid-m65-XXXXXX)"
export LIQUID_FORMAT=json
```

#### 1. Bootstrap + workspace create

```sh
liquid workspace create demo-walkthrough | jq .
ls -la "$LIQUID_HOME"
```

**Expected:** `data.workspace_id` is a UUID, `data.name` is
`demo-walkthrough`, `ok` is `true`. `ls -la $LIQUID_HOME` shows
six entries on first run: `auth/`, `vcs/`, `perm/`, `registry/`,
`secret`, `token`. The `secret` file is ≥ 16 bytes
(`wc -c < $LIQUID_HOME/secret` ≥ 16). The `token` is one line
matching `^u:[0-9a-f-]+\.[0-9]+\.[0-9a-f]+$` (bootstrap user,
not agent).

**Regression shape:** if any of the six files is missing, the
bootstrap sequence in `core/liquid-cli/src/services.rs::build_services`
+ `core/liquid-cli/src/token.rs::bootstrap` has drifted.

#### 2. Provision an agent + capture its token

```sh
WS=$(liquid workspace create real-workspace | jq -r .data.workspace_id)
liquid auth provision-agent demo-bot \
  --workspace "$WS" --role WorkspaceMember | jq .
```

**Expected:** `data.token` matches `^a:[0-9a-f-]+\.[0-9]+\.[0-9a-f]+$`
(`a:` prefix means agent); `data.role` is `"WorkspaceMember"`;
`data.agent_id` is a UUID; `ok` is `true`.

Capture for the next step:

```sh
export LIQUID_TOKEN=$(liquid auth provision-agent demo-bot2 \
  --workspace "$WS" --role WorkspaceMember | jq -r .data.token)
```

#### 3. Page write + read round-trip

```sh
liquid page write /pages/welcome --workspace "$WS" \
  --data '{"title":"hello","body":"world"}' | jq .
liquid page read /pages/welcome --workspace "$WS" | jq .
```

**Expected:** the `write` envelope's `data` has `path`,
`commit_id`, `operation_id` (all non-empty). The `read` envelope's
`data` is the JSON you wrote (`title` = `"hello"`, `body` =
`"world"`). The bytes are persisted under
`$LIQUID_HOME/vcs/<ws-uuid>/files/pages/welcome`.

#### 4. Audit list

```sh
liquid audit list --workspace "$WS" | jq .
```

**Expected:** one NDJSON object per write you performed in §3,
oldest-first (so `tail -n 1` returns the newest). Each row has
`action="Write"`, `path="/pages/welcome"`, `principal` matching
`^a:[0-9a-f-]+$` (the agent that wrote it), plus
`operation_id`, `commit_id`, `timestamp_unix_millis`, `message`.

Try the `--action` filter:

```sh
liquid audit list --workspace "$WS" --action Write | jq .action
```

**Expected:** every line emits `"Write"`. Drop `--action Write`
and try `--action Update` — only overwrites of an existing path
appear (zero in this walkthrough; we only wrote one path once).

#### 5. Undo + verify NotFound

```sh
OP=$(liquid page write /pages/welcome --workspace "$WS" \
       --data '{"v":2}' | jq -r .data.operation_id)
liquid page undo /pages/welcome --workspace "$WS" --op "$OP" | jq .
liquid page read /pages/welcome --workspace "$WS" | jq .
echo "read exit: $?"
```

**Expected:** the `undo` envelope's `data` has `commit_id`
(synthetic). The post-undo `read` exits non-zero (1) with
`ok: false, error: "Not found: ..."`. The error message must NOT
leak the internal `vcs/...` path — only a user-friendly
"Not found" string.

#### 6. Negative path: AppViewer cannot write

```sh
SCOPE=$(uuidgen)
VIEWER_TOKEN=$(liquid auth provision-agent demo-viewer \
  --workspace "$WS" --role AppViewer --scope "$SCOPE" \
  | jq -r .data.token)
LIQUID_TOKEN="$VIEWER_TOKEN" liquid page write /pages/x \
  --workspace "$WS" --data '{"v":1}' | jq .
echo "exit: $?"
```

**Expected:** exit `1`, `ok: false`, `error: "Forbidden"`.
The error is the literal string `"Forbidden"` (Absolute Rule
4.5 — never leak which auth mode failed). If you instead see
`"Not found"`, the rejection happened in the wrong place
(post-permission, not at the gate); reject the regression with a
citation back to Absolute Rule 4 + ADR-004.

#### 7. Cleanup

```sh
liquid auth token | jq -r .data.token | head -c 16; echo "…"
rm -rf "$LIQUID_HOME"
```

The token print proves the bearer is still resolvable after all
the prior commands; the cleanup restores a clean dir.

### Step M6.5.3 — Surface invariants by inspection

```sh
grep -nE 'pub async fn (create|provision|token|write|read|undo|list)' \
  core/liquid-cli/src/cmd/*.rs
```

Confirm by eye, for each subcommand handler:

1. The first executable line is `token::require(home)?` (or
   `token::resolve(home)` for `workspace create`'s bootstrap
   path, then `bootstrap` fallback).
2. The next line is `services.identity.validate_token(&token).await?`.
3. For mutating arms (`page write`, `page undo`,
   `auth provision-agent`), the third line is
   `require_permission!(perms, principal, Action::…, Resource::…)`.
   The §M5.2 module-doc precedent applies — `list` filters
   per-row, `audit list` runs an explicit
   `permissions.check(_, Read, Workspace(_))`.

**Regression shape:** any handler that does state-touching work
before `validate_token` is a Rule-4 violation. Any handler that
mutates state without `require_permission!` (or the explicit
`check` equivalent) is the same.

### Step M6.5.4 — Lints + format

```sh
cargo clippy --manifest-path core/Cargo.toml -p liquid-cli \
  --all-targets --locked -- -D warnings
cargo fmt --manifest-path core/Cargo.toml --all --check
```

**Expected:** no warnings. The workspace lint config forbids
`unwrap` / `expect` / `panic` outside `#[cfg(test)]`; the CLI
honours it everywhere (the only `unwrap_or_default` is on the
on-disk-path tmpfile-name fallback, which is an `OsString`
default — not an Absolute-Rule-1 violation).

### Step M6.5.5 — Cross-process persistence smoke

```sh
export LIQUID_HOME="$(mktemp -d)"
WS=$(liquid workspace create alpha | jq -r .data.workspace_id)
# Second invocation, fresh process, must see the same workspace:
liquid auth token  # bootstrap token from disk, no re-bootstrap
liquid audit list --workspace "$WS" | head
```

**Expected:** the audit list responds (even if empty), proving
the second process resolved the bootstrap token, opened the
filesystem registry, and could authenticate the call.

---

## Sign-off checklist

Tick every box before stamping the run-log:

- [ ] M6.5 — Step M6.5.1 reports 6 / 0 (mvp_slice) + 16 / 0
      (cli_subcommands) + the bridge tests at their §M5 baseline.
- [ ] M6.5 — Step M6.5.2's seven walkthrough commands all
      produce the documented envelope shapes; exit codes match.
- [ ] M6.5 — Step M6.5.3 grep confirms every handler is
      `token::require` → `validate_token` → `require_permission!`
      (with the three documented exceptions).
- [ ] M6.5 — Step M6.5.4 reports clippy + fmt clean.
- [ ] M6.5 — Step M6.5.5 cross-process smoke succeeds.
- [ ] `just deny-check` clean.
- [ ] `just coverage-check` clean (`core/liquid-cli/**` excluded
      via `--exclude-files` per §15; behaviour proven by bats).

If any line above is unchecked, the milestone is **not** done; do
not tag the release.

---

## Related documents

- [`manual-validation-m1-m3.md`](manual-validation-m1-m3.md) — M1
  primitives + M2 VCS + M3 auth+permissions, the four
  backends this CLI composes.
- [`manual-validation-m4-m5.md`](manual-validation-m4-m5.md) — M4
  cache + M5 Rust-side FFI bridge (`BridgeServices`).
- `IMPLEMENTATION_PLAN.md §5.6` — milestone spec (sub-bullets
  ticked).
- `IMPLEMENTATION_PLAN.md §9` `liquid-cli` — composition + state
  layout reference.
- `IMPLEMENTATION_PLAN.md §12` — full CLI grammar; M6.5 ships
  the §5.6 subset, M7 ships the rest.
- `docs/adr/004-bridge-token-first-arg.md` — the token-first
  pattern the CLI uses at every call site.
- `tests/cli/00_mvp_slice.bats` + `tests/cli/10_cli_subcommands.bats`
  — the live regression gates this guide walks.
- `CHANGELOG.md` — every M6.5 surface change ships with a
  matching `## [Unreleased]` entry.
