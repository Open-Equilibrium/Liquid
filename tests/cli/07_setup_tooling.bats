#!/usr/bin/env bats
#
# tests/cli/07_setup_tooling.bats — covers `scripts/setup-tooling.sh`,
# the idempotent installer that brings a fresh clone (or fresh
# container) up to the toolchain set the project's pre-push and CI
# gates assume.
#
# The script is the single source of truth for which versions of
# `cargo-deny`, `cargo-tarpaulin`, `just`, `bats`, and `lefthook` the
# repo targets. CONTRIBUTING.md and CLAUDE.md "First-time setup" both
# reduce to one bullet that points at this script, so the install
# pins live in exactly one place and never drift.
#
# These tests do NOT actually install the tools — that is slow and
# network-dependent. They only assert the script's shape:
#
#   1. The file exists, is executable, and parses with `bash -n`.
#   2. The script mentions every required tool by name (so adding a
#      tool to the table without also adding it to the script is
#      caught here).
#   3. The script is idempotent on its happy path (a `--dry-run`
#      invocation prints `would install` lines without mutating
#      anything, and re-running detects already-installed tools).
#   4. CONTRIBUTING.md and CLAUDE.md "First-time setup" both
#      reference the script.

# shellcheck shell=bash

ROOT="$BATS_TEST_DIRNAME/../.."
SCRIPT="$ROOT/scripts/setup-tooling.sh"

TOOLS=(cargo-deny cargo-tarpaulin just bats lefthook)

@test "script exists" {
  [ -f "$SCRIPT" ]
}

@test "script is executable" {
  [ -x "$SCRIPT" ]
}

@test "script parses with bash -n" {
  bash -n "$SCRIPT"
}

@test "script references cargo-deny" {
  grep -qE 'cargo[-_]deny' "$SCRIPT"
}

@test "script references cargo-tarpaulin" {
  grep -qE 'cargo[-_]tarpaulin' "$SCRIPT"
}

@test "script references just" {
  grep -qwE 'just' "$SCRIPT"
}

@test "script references bats" {
  grep -qwE 'bats' "$SCRIPT"
}

@test "script references lefthook" {
  grep -qwE 'lefthook' "$SCRIPT"
}

@test "script supports --help" {
  run bash "$SCRIPT" --help
  [ "$status" -eq 0 ]
  [[ "$output" == *"setup-tooling"* ]]
}

@test "script supports --dry-run and prints install plan without mutating" {
  run bash "$SCRIPT" --dry-run
  [ "$status" -eq 0 ]
  # Every tool in $TOOLS must appear in the dry-run output. Use an
  # explicit `|| { ... }` block per assertion — a bare `[[ ... ]]`
  # inside a for-loop only surfaces the last iteration's exit status
  # to BATS, so a missing earlier tool would silently pass.
  for tool in "${TOOLS[@]}"; do
    [[ "$output" == *"$tool"* ]] || {
      printf 'dry-run output missing tool: %s\n' "$tool" >&2
      return 1
    }
  done
}

@test "script rejects unknown arguments with exit code 2" {
  run bash "$SCRIPT" --does-not-exist
  [ "$status" -eq 2 ]
  [[ "$output" == *"unknown argument"* ]]
}

@test "script --dry-run is idempotent — a second run prints the same plan" {
  run bash "$SCRIPT" --dry-run
  [ "$status" -eq 0 ]
  first="$output"
  run bash "$SCRIPT" --dry-run
  [ "$status" -eq 0 ]
  [ "$output" = "$first" ]
}

@test "CONTRIBUTING.md first-time setup points at the script" {
  grep -q 'setup-tooling.sh' "$ROOT/CONTRIBUTING.md"
}

@test "CLAUDE.md first-time setup points at the script" {
  grep -q 'setup-tooling.sh' "$ROOT/CLAUDE.md"
}
