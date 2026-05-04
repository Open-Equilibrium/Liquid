#!/usr/bin/env bash
set -euo pipefail

# Captures git status + diffstat snapshots after Edit/Write tool use.
# Wired from .claude/settings.json (PostToolUse on Edit|Write).

mkdir -p .ai/artifacts/diffs .ai/artifacts/logs .ai/artifacts/ui

ts="$(date -u +%Y%m%dT%H%M%SZ)"

if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git status --short > ".ai/artifacts/diffs/status-${ts}.txt" || true
  git diff --stat > ".ai/artifacts/diffs/diffstat-${ts}.txt" || true
fi

echo "Artifacts directory ready: .ai/artifacts/"
