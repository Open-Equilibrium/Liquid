#!/usr/bin/env bash
# SessionStart hook: warms toolchains and surfaces a brief project snapshot.
# Wired from .claude/settings.json. Runs once at the start of every Claude
# Code session. Stays quiet unless something is unhealthy.
#
# Design principles:
# - Never fail the session start. All commands are best-effort.
# - Skip layers whose code does not exist yet (matches lefthook + CI gates).
# - Do not fetch over the network if cargo registry is already warm.
# - Cap total runtime; the user is waiting.

set -uo pipefail

mkdir -p .ai/artifacts/logs

ts="$(date -u +%Y%m%dT%H%M%SZ)"
log=".ai/artifacts/logs/session-start-${ts}.log"

{
  echo "Liquid session start @ ${ts}"
  echo

  if command -v cargo >/dev/null 2>&1; then
    cargo --version
  fi
  if command -v rustc >/dev/null 2>&1; then
    rustc --version
  fi
  if command -v flutter >/dev/null 2>&1; then
    flutter --version 2>/dev/null | head -1
  fi
  if command -v dart >/dev/null 2>&1; then
    dart --version 2>&1 | head -1
  fi
  if command -v bats >/dev/null 2>&1; then
    bats --version 2>/dev/null
  fi
  if command -v just >/dev/null 2>&1; then
    just --version
  fi

  echo
  echo "Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
  echo "HEAD:   $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"

  if [ -f core/Cargo.toml ] && command -v cargo >/dev/null 2>&1; then
    echo
    echo "Warming cargo registry (best-effort, offline-tolerant)..."
    timeout 30 cargo fetch --manifest-path core/Cargo.toml --locked >/dev/null 2>&1 \
      && echo "  cargo fetch: ok" \
      || echo "  cargo fetch: skipped (offline or already warm)"
  fi
} > "$log" 2>&1 || true

# Print a one-line greeting; details live in the log.
toolchain="$( (cargo --version 2>/dev/null || echo no-cargo) | awk '{print $1, $2}')"
branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
echo "Liquid ready · branch=${branch} · ${toolchain} · log=${log}"

# ── Compact resume snapshot (chat side, capped at 30 lines) ────────────────
# Print a small, signal-only block so a resume turn does not have to
# Read the obvious files to re-establish context. Sections:
#
#   * branch (echoed again so the block stands alone if the greeting
#     scrolls off)
#   * `git status --short` of the working tree (uncommitted edits +
#     untracked files at a glance)
#   * the 5 most-recently-modified tracked files (so the agent can
#     orient on the conversation's likely focus)
#
# Each section is capped; the whole block stays well under 30 lines so
# resume noise stays bounded. Outside a git repo (or with no commits
# yet) every section silently no-ops — the canonical greeting above
# is the only required output.
if git rev-parse --git-dir >/dev/null 2>&1; then
  echo
  echo "On branch: ${branch}"

  short_status="$(git status --short 2>/dev/null | head -15 || true)"
  if [ -n "$short_status" ]; then
    echo "Working tree:"
    printf '%s\n' "$short_status"
  else
    echo "Working tree: clean"
  fi

  recent="$(git log --pretty=format: --name-only --no-merges -n 25 2>/dev/null \
            | awk 'NF && !seen[$0]++ { print; n++; if (n==5) exit }' \
            || true)"
  if [ -n "$recent" ]; then
    echo "Recently modified:"
    printf '  %s\n' $recent
  fi
fi
