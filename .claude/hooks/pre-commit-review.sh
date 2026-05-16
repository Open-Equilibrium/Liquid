#!/usr/bin/env bash
# .claude/hooks/pre-commit-review.sh — Claude Code `PreToolUse` hook
# matched on `Bash(git commit*)`.
#
# When the agent is about to run `git commit ...`, this hook is invoked
# with the tool-call payload on stdin (JSON, single line). The hook:
#
#   1. Extracts the about-to-run `git commit` command from the payload.
#   2. Reads `git diff --staged`.
#   3. Spawns the `code-reviewer` subagent (haiku, read-only — see
#      `.claude/agents/code-reviewer.md`) by emitting a hookSpecificOutput
#      message that the harness routes back to the main agent.
#   4. Blocks (`decision: "block"`) only if the spawned reviewer reports
#      a critical finding. Warnings and suggestions do not block —
#      they are surfaced as `additionalContext` for the agent to read
#      and act on at its discretion.
#
# Design notes:
#
# - The hook itself does NOT call Claude — it cannot, because the
#   Claude Code harness does not let hook scripts originate inference.
#   What it CAN do is return a JSON envelope that:
#     a) carries the staged diff into the agent's next turn as
#        `additionalContext`, and
#     b) sets `decision: "ask"` so the harness asks the agent to spawn
#        the `code-reviewer` subagent before the commit lands.
#   The agent's existing per-commit discipline (CLAUDE.md "TDD
#   discipline") is what actually invokes the subagent; the hook just
#   guarantees the discipline cannot be skipped by accident.
#
# - When the staged diff is empty (e.g. `git commit --allow-empty`,
#   amend with no new staged changes), the hook is a no-op and lets
#   the commit through.
#
# - The hook respects an opt-out via `LIQUID_SKIP_PRE_COMMIT_REVIEW=1`
#   in the agent's environment. Use it for trivial rebases or
#   conflict-resolution commits where reviewing the staged diff is
#   not informative. Document the use in the commit body so reviewers
#   can spot abuse.
#
# - The hook writes a snapshot of the staged diff to
#   `.ai/artifacts/diffs/pre-commit-<ts>.diff` so the agent has a
#   stable path to point the subagent at, independent of subsequent
#   `git add` / `git restore` calls.

set -euo pipefail

# Opt-out for the rare case where review is genuinely uninformative.
if [ "${LIQUID_SKIP_PRE_COMMIT_REVIEW:-0}" = "1" ]; then
  printf '{"continue": true}\n'
  exit 0
fi

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
diffs_dir="$repo_root/.ai/artifacts/diffs"
mkdir -p "$diffs_dir"
ts=$(date -u +%Y%m%dT%H%M%SZ)
diff_path="$diffs_dir/pre-commit-${ts}.diff"
stat_path="$diffs_dir/pre-commit-${ts}.stat"

# Snapshot the staged diff. If empty, let the commit through.
git -C "$repo_root" diff --staged > "$diff_path" 2>/dev/null || true
git -C "$repo_root" diff --staged --stat > "$stat_path" 2>/dev/null || true

if [ ! -s "$diff_path" ]; then
  rm -f "$diff_path" "$stat_path"
  printf '{"continue": true}\n'
  exit 0
fi

# Compact summary line for the hookSpecificOutput message — keeps the
# main-thread payload small while pointing the agent at the full diff.
files_changed=$(wc -l < "$stat_path" | tr -d ' ')
diff_lines=$(wc -l < "$diff_path" | tr -d ' ')

# We do NOT block the tool call here. Instead we return `decision:
# "ask"` so the harness asks the agent to confirm — and the
# CLAUDE.md-mandated discipline tells the agent to spawn the
# `code-reviewer` subagent on `git diff --staged` before answering.
# The subagent is the one that decides whether a critical finding
# exists; the hook merely guarantees the conversation happens.
cat <<EOF
{
  "decision": "ask",
  "reason": "Per-commit review gate: spawn the \`code-reviewer\` subagent on \`git diff --staged\` and block this commit if it reports any \`critical\` finding. Warnings and suggestions are advisory.",
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "additionalContext": "Staged diff snapshot: ${diff_path} (${diff_lines} lines, ${files_changed} files changed). Run the \`code-reviewer\` subagent against it; commit only if the JSON response has an empty \`critical\` array. To opt out for a trivial rebase / conflict-resolution commit, re-issue the commit with LIQUID_SKIP_PRE_COMMIT_REVIEW=1 and explain why in the commit body."
  }
}
EOF
