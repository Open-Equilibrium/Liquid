# `tests/cli/` — bats integration suite for the `liquid` CLI

This directory is the **server-side acceptance suite** for the `liquid`
CLI. Per `CLAUDE.md` Absolute Rule 6 ("CLI before UI"), every
data-touching feature must be reachable via the CLI — and proven via a
bats test here — before any UI work begins.

## Current status: skip-only (M6.5 pending)

> **Reading this before M6.5 ships?** Almost every test here is a
> `skip "pending M6.5"`. That is intentional, not bit-rot.

The MVP CLI surface (`workspace create`, `auth provision-agent`,
`auth token`, `page write`, `page read`, `audit list`, `page undo`)
lands in TASK-008 (M6.5 — `IMPLEMENTATION_PLAN.md` §5.6). Until that
task is closed, the spec lives here as a `skip`-guarded scaffold so
that:

1. The shape of the eventual end-to-end test is committed and
   reviewable — the contract is agreed before implementation, not
   discovered during it.
2. CI exercises the suite (`bats tests/cli/`) on every PR, so when a
   skip is dropped the test runs immediately. There is no separate
   "switch on" step.
3. Drift between the documented CLI grammar
   (`IMPLEMENTATION_PLAN.md` §12) and the suite is caught by reading
   one file, not by reading the whole CLI.

Concretely, **a green `bats tests/cli/` run today proves the suite is
syntactically valid and that no test panics; it does NOT prove the
CLI itself behaves correctly.** Until M6.5, the only files that
actually exercise live code are:

- `01_branch_name_gate.bats` — exercises
  `scripts/check-branch-name.sh` (the pre-push branch-name gate). This
  test is **live**: removing the `skip` was never there, and a
  regression in the gate fails the suite.

Every other `.bats` file in this directory should be treated as a
specification until M6.5 closes.

## How to be honest about coverage in PRs

The PR template (`.github/PULL_REQUEST_TEMPLATE.md`) carries a
"Coverage claim" checkbox. When you tick the "CLI integration test
exists" line, also state explicitly whether the test is **live** or
**skip-pending-M6.5**. Examples:

- "Added `tests/cli/05_workspace_create.bats` — live, asserts
  `workspace create` happy path + missing-token rejection."
- "Updated `tests/cli/00_mvp_slice.bats` step 3 — still
  `skip "pending M6.5"`; the spec text now matches the new flag."

Reviewers will reject "CLI test added" claims that turn out to be
all-`skip`.

## Running locally

```sh
just test-cli            # or: bats tests/cli/
just test-cli-filtered   # raw log to .ai/artifacts/logs/, summary to stdout
```

`bats` install hint:
<https://bats-core.readthedocs.io/en/stable/installation.html>.

## Layout convention

- `00_mvp_slice.bats` — the single canonical end-to-end spec for the
  Phase-1 happy path (M6.5 milestone test).
- `01_…`, `02_…` etc. — focused suites for individual subcommands
  or gates, ordered by execution priority.
- Helpers (when added) live under `tests/cli/helpers/` and are
  loaded with `load helpers/<name>`.

## Why CLI before UI

The CLI is the smallest, most stable surface that exercises every
Rust crate end-to-end. Proving a feature via bats before any Flutter
work means:

- Agents can drive the feature without an emulator.
- Regressions are caught at the layer with the lowest test cost.
- The UI is a thin renderer over a tested data path, not the place
  where new bugs land.
