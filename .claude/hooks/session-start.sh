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
