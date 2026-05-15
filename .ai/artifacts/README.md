# `.ai/artifacts/` — Agent run-time output

This tree holds session-local output produced by Claude Code (and other
local AI agents) while they work on this repo. **Nothing in this tree is
committed.** The repo-root `.gitignore` ignores `.ai/` wholesale; see
`# ─── AI / agent artifacts ───` in [`/.gitignore`](../../.gitignore).

The tree exists so the main chat thread stays small: noisy command
output, diffs, golden snapshots, screenshots, and traces land here, and
only a compact summary surfaces in conversation. Agents reference these
paths to the human or to follow-up subagents.

## Layout

| Subtree | What lives here | Producer |
|---|---|---|
| `logs/` | Raw test/lint/build logs from cargo, flutter, bats, analyzer. Sized like the original — possibly tens of MB. | `.claude/hooks/filter-test-output.sh`, `.claude/hooks/session-start.sh`, any `just *-filtered` recipe. |
| `diffs/` | Snapshots of `git status` / `git diff --stat` taken after `Edit` / `Write` tool calls. Useful for recovering what an agent touched mid-session. | `.claude/hooks/save-artifacts.sh` (`PostToolUse` hook). |
| `ui/` | Flutter widget screenshots, golden diffs, and any visual output produced by the `ui-validator` subagent. | `ui-validator` subagent (`.claude/agents/ui-validator.md`). |

If you add a new subtree, document its purpose and producer in the table
above so the next agent (or the next human) can tell at a glance what to
keep and what to nuke.

## Naming convention

Filenames are prefixed with an ISO-8601 UTC timestamp so chronological
order matches lexical order:

```
.ai/artifacts/logs/raw-20260515T194434Z.log
.ai/artifacts/logs/summary-20260515T194434Z.log
.ai/artifacts/diffs/diff-20260515T194512Z.txt
.ai/artifacts/ui/screenshot-20260515T194600Z-grid_drag.png
```

`filter-test-output.sh` already emits `raw-<ts>.log` and
`summary-<ts>.log`. New producers should follow the same pattern.

## Retention

- **Per session:** keep everything. The point of the tree is to be a
  scratchpad for the active session.
- **Between sessions:** safe to delete the entire `.ai/artifacts/` tree
  at any time. Nothing here is load-bearing; if a future agent needs
  the data, it should regenerate it (re-run the test, take the
  screenshot again, etc.).
- **Cloud Claude Code:** the execution container is reclaimed when the
  session ends, so the tree is effectively wiped automatically. No
  retention action required from the agent.
- **Local Claude Code:** the human is expected to garbage-collect this
  tree periodically. A simple `rm -rf .ai/artifacts/{logs,diffs,ui}/*`
  is fine; it never breaks anything.

## .gitignore behaviour

Root [`.gitignore`](../../.gitignore) contains:

```gitignore
# ─── AI / agent artifacts ──────────────────────────────────────────────
# .claude/ is the repo-local AI agent config — it IS tracked.
# .ai/ holds run-time agent artefacts (raw logs, diffs, screenshots) —
# these are session-local and should not be committed.
.ai/
```

This README is the only tracked file under `.ai/`. It exists to
document the convention; `git add -f .ai/artifacts/README.md` is the
one-time command that brought it in.

## Why a tracked README instead of an empty `.gitkeep`?

Without explicit documentation, agents tend to either:

1. Treat `.ai/artifacts/` as scratch and dump raw output into the chat
   anyway (because the convention isn't obvious), or
2. Treat the tree as load-bearing and start committing screenshots into
   the repo "for posterity" (because the retention policy isn't
   obvious).

This README codifies the answer to both: dump everything here, commit
nothing.
