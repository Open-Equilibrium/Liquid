# Subagent Routing

**Every recurring class of work that a specialised subagent can
handle MUST be delegated to that subagent, not done inline.** The
main agent's job is orchestration; subagents are the workforce.
Doing their work in the main thread burns the main context window
and silently drops the per-agent context-isolation benefit.

## Decision table

| Trigger | Route to | Notes |
|---|---|---|
| Any `mcp__github__*` READ — `list_*`, `get_*`, `*_read`, `search_*` (PRs, issues, branches, commits, releases, CI status) | `github-pr` (haiku, read-only) | Returns a compact summary; the main agent decides whether to act. Cannot push, comment, merge, or modify state — write calls (`mcp__github__add_issue_comment`, `mcp__github__create_pull_request`, etc.) MUST stay in the main agent. |
| `cargo test`, `cargo clippy`, `flutter test`, `bats tests/cli/` output >50 lines (esp. failure cascades) | `test-triager` (haiku, read-only) | Parses the log offline and returns just the failing test, file path, first meaningful error, and the next smallest command. Pair with `.claude/hooks/filter-test-output.sh` for single-failure cases. |
| "Where is X defined / which files reference Y" lookups that span >3 grep candidates or unclear naming | `Explore` (general-purpose) | Direct `grep`/`rg` for known names; spawn `Explore` for open-ended search. See [`api-grep-discipline.md`](api-grep-discipline.md). |
| Every staged-diff commit (`git diff --staged` after `git add`) | `code-reviewer` (sonnet, read-only) | Mandatory per `.claude/hooks/pre-commit-review.sh`. A non-empty `critical[]` array blocks the commit; warnings + suggestions are advisory. Skippable for a single conflict-resolution commit via `[skip-review]` trailer or `LIQUID_SKIP_PRE_COMMIT_REVIEW=1`. |
| Flutter widget / integration / golden / screenshot validation | `ui-validator` (sonnet, read-only) | Falls back to widget-level tests in cloud sessions without an emulator. Never adds Playwright. |
| GitHub Actions failed run, log >50 lines | `.claude/scripts/gh-job-log <run_id>` | Not a subagent — a script. Writes raw log to `.ai/artifacts/logs/`, prints last 50 lines of every failed step. See [`log-volume.md`](log-volume.md). |

## When NOT to delegate

- Single targeted grep with a known file path and symbol — use the
  `Bash` tool directly. Spawning `Explore` adds latency.
- A `mcp__github__*` WRITE — `add_issue_comment`,
  `create_pull_request`, `pull_request_review_write`,
  `update_pull_request`, `merge_pull_request`, `push_files`, etc.
  These mutate state and must NOT be hidden behind a subagent
  delegation — the main agent is accountable.
- A two-line cargo output that fits in 50 lines and has one obvious
  failure — read it directly. `test-triager` is for noise, not for
  the common case.

## Why this matters

Two failure modes the table prevents:

1. **Context burn.** Pasting a 400-line cargo cascade into the main
   thread drowns useful signal and inflates downstream cost.
   `test-triager` returns 10 lines.
2. **Tool sprawl.** Each `mcp__github__*` read call returns a JSON
   payload that grows with PR history. Delegating to `github-pr`
   keeps that JSON in a side context the main thread never loads.

## Hard rule

If a request matches one of the table rows above and you find
yourself about to do that work inline, **stop**. Delegate. The
exception list ("When NOT to delegate") is exhaustive; if your case
is not on it, the work belongs in a subagent.
