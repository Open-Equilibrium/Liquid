#!/usr/bin/env bash
# .claude/hooks/pre-commit-review.sh — Claude Code `PreToolUse` hook
# matched on `Bash(git commit*)`.
#
# When the agent is about to run `git commit ...`, this hook:
#
#   1. Reads the tool-call payload from stdin (JSON) — the harness
#      hands us the full `command` string the agent intends to run.
#   2. Snapshots `git diff --staged` to
#      `.ai/artifacts/diffs/pre-commit-<ts>.diff`.
#   3. Returns a `hookSpecificOutput.permissionDecision = "ask"`
#      response so the harness asks the agent to confirm before the
#      commit lands.
#   4. The `code-reviewer` subagent (haiku, read-only — see
#      `.claude/agents/code-reviewer.md`) is the gate: the agent must
#      spawn it on the snapshot and block the commit if the JSON
#      response carries a non-empty `critical` array. Warnings and
#      suggestions are advisory.
#
# Design notes:
#
# - The hook itself does NOT call Claude — it cannot, because the
#   Claude Code harness does not let hook scripts originate inference.
#   It returns a structured JSON envelope; the harness routes the
#   `permissionDecisionReason` text into the agent's next turn so the
#   agent sees the snapshot path and instructions.
#
# - When the staged diff is empty (e.g. `git commit --allow-empty`,
#   amend with no new staged changes, or `git commit-tree` /
#   `git commit-graph` plumbing matches), the hook silently returns
#   `{"continue": true}` and lets the call through.
#
# - Two opt-out paths:
#     a. Set `LIQUID_SKIP_PRE_COMMIT_REVIEW=1` in the host
#        environment BEFORE starting Claude Code. The hook reads its
#        own environment and short-circuits. Prefer this for a long
#        rebase session.
#     b. Include the literal token `[skip-review]` anywhere in the
#        commit message of the about-to-run `git commit -m "..."`.
#        The hook parses the command from stdin and short-circuits.
#        Prefer this for a single conflict-resolution commit.
#   Document the reason in the commit body so reviewers can spot
#   abuse.
#
# - Race window: the snapshot is taken at PreToolUse time. If the
#   agent stages more files between the snapshot and the actual
#   commit, the code-reviewer will be reviewing a stale diff. This
#   is inherent to any PreToolUse "ask" flow. The harness re-fires
#   the hook on each retry of `git commit`, so re-staging then
#   re-committing produces a fresh snapshot.

set -euo pipefail

# ── 1. Host-env opt-out (set before starting Claude Code). ───────────
if [ "${LIQUID_SKIP_PRE_COMMIT_REVIEW:-0}" = "1" ]; then
  printf '{"continue": true}\n'
  exit 0
fi

# ── 2. Parse the tool-call command from stdin. ───────────────────────
# Reading stdin is best-effort — if jq isn't available or stdin is
# empty (e.g. running the hook directly from a shell for testing), we
# fall through and use the staged-diff-only path.
payload=""
if [ ! -t 0 ]; then
  payload=$(cat || true)
fi
cmd=""
if [ -n "$payload" ] && command -v jq >/dev/null 2>&1; then
  cmd=$(printf '%s' "$payload" | jq -r '.tool_input.command // empty' 2>/dev/null || true)
fi

# ── 2a. Per-commit opt-out via [skip-review] token in the message. ───
if [ -n "$cmd" ] && printf '%s' "$cmd" | grep -qF '[skip-review]'; then
  printf '{"continue": true}\n'
  exit 0
fi

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
diffs_dir="$repo_root/.ai/artifacts/diffs"
mkdir -p "$diffs_dir"
ts=$(date -u +%Y%m%dT%H%M%SZ)
diff_path="$diffs_dir/pre-commit-${ts}.diff"
stat_path="$diffs_dir/pre-commit-${ts}.stat"

# ── 3. Snapshot the staged diff. Empty → silent allow. ───────────────
git -C "$repo_root" diff --staged > "$diff_path" 2>/dev/null || true
git -C "$repo_root" diff --staged --stat > "$stat_path" 2>/dev/null || true

if [ ! -s "$diff_path" ]; then
  rm -f "$diff_path" "$stat_path"
  printf '{"continue": true}\n'
  exit 0
fi

# ── 4. Compact summary numbers. ─────────────────────────────────────
# `git diff --stat` always emits a trailing summary line
# ("N files changed, X insertions(+), Y deletions(-)"), so subtract
# 1 from the wc -l count to get the true file count.
stat_lines=$(wc -l < "$stat_path" | tr -d ' ')
files_changed=$(( stat_lines > 0 ? stat_lines - 1 : 0 ))
diff_lines=$(wc -l < "$diff_path" | tr -d ' ')

# ── 5. Best-effort tail cleanup so the diffs/ tree doesn't grow ─────
# unbounded across a long session. Keep the most recent 20 snapshots.
ls -1t "$diffs_dir"/pre-commit-*.diff 2>/dev/null | tail -n +21 | xargs -r rm -f || true
ls -1t "$diffs_dir"/pre-commit-*.stat 2>/dev/null | tail -n +21 | xargs -r rm -f || true

# ── 6. Emit the PreToolUse hookSpecificOutput envelope. ──────────────
# Schema: `permissionDecision: "ask"` defers to the agent + user, with
# `permissionDecisionReason` injected as context. This is the documented
# PreToolUse contract (see Claude Code hooks docs).
#
# We use jq for JSON construction so embedded paths / quotes are
# escaped correctly. Falling back to a hand-built string would be
# fragile for any path containing `"` or `\`.
if command -v jq >/dev/null 2>&1; then
  jq -n \
    --arg snap "$diff_path" \
    --arg lines "$diff_lines" \
    --arg files "$files_changed" \
    '{
      "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "ask",
        "permissionDecisionReason": ("Per-commit review gate. Snapshot: " + $snap + " (" + $lines + " lines, " + $files + " files). Spawn the `code-reviewer` subagent on this snapshot before confirming the commit; block if the JSON response has a non-empty `critical` array. Opt-out: include `[skip-review]` in the commit message, or set LIQUID_SKIP_PRE_COMMIT_REVIEW=1 in the host env before starting Claude Code.")
      }
    }'
else
  # jq absent — emit a minimal continue:true so the hook never hard-
  # blocks a commit just because jq isn't installed. The discipline
  # falls back to the agent's CLAUDE.md-mandated workflow.
  printf '{"continue": true}\n'
fi
