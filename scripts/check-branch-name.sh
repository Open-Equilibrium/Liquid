#!/usr/bin/env bash
# scripts/check-branch-name.sh — pre-push gate.
#
# Rejects pushes whose current branch is either:
#   - `main`                 (direct pushes to the integration branch
#                             are policy-violations; use a PR)
#   - matches `^claude/...`  (the Claude Code agent autobranch
#                             namespace; agents must rebase onto a
#                             `feature/...` branch and push there)
#
# Usage:
#   scripts/check-branch-name.sh            # reads `git rev-parse --abbrev-ref HEAD`
#   scripts/check-branch-name.sh <branch>   # tests an explicit branch name (used by bats)
#
# Wired in via lefthook.yml's `pre-push` hook. CLAUDE.md "Mandatory
# Development Workflow" and the per-session goal both forbid pushing to
# `main` or `claude/*`; this script is the local enforcement.

set -euo pipefail

# Distinguish "no argument at all" (use git detection) from "argument is
# the empty string" (caller bug — surface it). Otherwise an explicit
# `bash check-branch-name.sh ""` would silently fall through to git
# detection and test the wrong thing.
if [ "$#" -gt 0 ]; then
  if [ -z "$1" ]; then
    printf 'check-branch-name: empty branch argument; pass a name or no argument\n' >&2
    exit 2
  fi
  branch="$1"
else
  branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || true)
fi

if [ -z "$branch" ]; then
  printf 'check-branch-name: could not determine current branch (not in a git repo?)\n' >&2
  exit 2
fi

# Exact-match `main` (not substring): `feat/main-page-redesign` is fine.
if [ "$branch" = "main" ]; then
  printf 'check-branch-name: refusing to push to "main".\n' >&2
  printf '  Open a feature branch (e.g. `feature/<topic>`) and push there;\n' >&2
  printf '  changes land on `main` via PR review, not direct push.\n' >&2
  exit 1
fi

# Reject the Claude Code agent autobranch namespace:
#   - bare `claude`            (a branch literally named `claude` is
#                               not a normal feature branch; the only
#                               reason to create it is by accident or
#                               as a placeholder for the namespace)
#   - `claude/` (trailing slash, normally impossible but guard anyway)
#   - `claude/<anything>` (including nested paths like `claude/a/b/c`;
#                          `*` in bash `case` matches `/` too)
# A branch like `feat/handle-claude-feedback` is fine because the
# `claude` prefix is not at position 0.
case "$branch" in
  claude|claude/|claude/*)
    printf 'check-branch-name: refusing to push branch "%s".\n' "$branch" >&2
    printf '  `claude` and `claude/*` are the Claude Code agent autobranch namespace;\n' >&2
    printf '  rebase onto a `feature/<topic>` (or `fix/<topic>`) branch and push there.\n' >&2
    exit 1
    ;;
esac

exit 0
