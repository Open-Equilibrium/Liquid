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
`liquid` CLI itself behaves correctly.** Until M6.5,
`00_mvp_slice.bats` is the only `skip`-guarded specification waiting
on the future `liquid` command surface.

The remaining suites cover the project's *agent-tooling* surface and
are **live today** — a regression in any of them fails the suite:

- `01_branch_name_gate.bats` — `scripts/check-branch-name.sh`
  (pre-push branch-name gate).
- `02_bump_version.bats` — `scripts/bump-version.sh`.
- `03_pre_commit_review_hook.bats` —
  `.claude/hooks/pre-commit-review.sh`.
- `04_changelog_gate.bats` — `.lefthook/commit-msg/check-changelog.sh`.
- `05_gh_job_log.bats` — `.claude/scripts/gh-job-log`.
- `06_coverage_recipes.bats` — `just coverage-check` / `just check-ci`.
- `07_setup_tooling.bats` — `scripts/setup-tooling.sh`.
- `08_clean_recipes.bats` — `just clean` / `just clean-walkthroughs`
  plus the M2 / M3 walkthrough refactors.
- `09_session_start_hook.bats` — `.claude/hooks/session-start.sh`.

Treat `00_mvp_slice.bats` (and any future test that ships behind a
`skip "pending M6.5"`) as specification; everything else here is
already part of the live gate.

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
