#!/usr/bin/env bash
set -euo pipefail

# Captures git status + diffstat snapshots after Edit/Write tool use.
# Wired from .claude/settings.json (PostToolUse on Edit|Write).
# Always writes under the repo root, never under cwd.

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
[ -n "$repo_root" ] || exit 0

cd "$repo_root"
mkdir -p .ai/artifacts/diffs .ai/artifacts/logs .ai/artifacts/ui

ts="$(date -u +%Y%m%dT%H%M%SZ)"
git status --short > ".ai/artifacts/diffs/status-${ts}.txt" || true
git diff --stat   > ".ai/artifacts/diffs/diffstat-${ts}.txt" || true
