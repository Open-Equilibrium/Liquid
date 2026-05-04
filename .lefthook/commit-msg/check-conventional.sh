#!/usr/bin/env bash
# Conventional Commits format check
# Called by lefthook commit-msg hook with the message file path as $1
set -euo pipefail

MSG=$(head -1 "$1")
PATTERN='^(feat|fix|docs|refactor|test|chore|perf)(\([a-z0-9_-]+\))?: .{1,100}$'

if ! echo "$MSG" | grep -qE "$PATTERN"; then
  echo "✗ Commit message rejected — does not follow Conventional Commits."
  echo "  Format:  <type>(<scope>): <summary>"
  echo "  Types:   feat  fix  docs  refactor  test  chore  perf"
  echo "  Scopes:  core vcs auth permissions cache bindings bridge cli app sdk registry ci deps"
  echo "  Example: feat(vcs): implement JujutsuContentStore read and write"
  echo ""
  echo "  Got: $MSG"
  exit 1
fi
