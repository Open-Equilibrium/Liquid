#!/usr/bin/env bats
#
# tests/cli/01_branch_name_gate.bats — verifies the pre-push branch-name
# gate rejects pushes from branches whose name is `main` or starts with
# `claude/` (the agent autobranch namespace). Agents must develop on
# `feature/*` (or similar) and open a PR; pushing directly to either of
# the forbidden patterns is a project-policy violation.
#
# The gate lives in scripts/check-branch-name.sh and is wired into
# `lefthook.yml`'s `pre-push` hook (CLAUDE.md "Mandatory Development
# Workflow").

# shellcheck shell=bash

GATE="$BATS_TEST_DIRNAME/../../scripts/check-branch-name.sh"

run_gate() {
  local branch="$1"
  # The gate reads its branch from the first positional arg when
  # provided; otherwise it falls back to `git rev-parse --abbrev-ref
  # HEAD`. Pass the branch explicitly so we never have to mutate the
  # repository's actual HEAD for a test.
  bash "$GATE" "$branch"
}

@test "gate exists and is executable" {
  [ -x "$GATE" ]
}

@test "rejects 'main'" {
  run run_gate main
  [ "$status" -ne 0 ]
  [[ "$output" == *"main"* ]]
}

@test "rejects 'claude/agent-tooling-hardening-r2-RQE8E'" {
  run run_gate "claude/agent-tooling-hardening-r2-RQE8E"
  [ "$status" -ne 0 ]
  [[ "$output" == *"claude"* ]]
}

@test "rejects bare 'claude/'" {
  run run_gate "claude/"
  [ "$status" -ne 0 ]
}

@test "rejects bare 'claude' (no slash) — autobranch namespace" {
  run run_gate "claude"
  [ "$status" -ne 0 ]
}

@test "rejects nested 'claude/a/b/c' — case glob crosses slashes" {
  run run_gate "claude/a/b/c"
  [ "$status" -ne 0 ]
}

@test "accepts 'feature/agent-tooling-hardening-r2'" {
  run run_gate "feature/agent-tooling-hardening-r2"
  [ "$status" -eq 0 ]
}

@test "accepts 'fix/cli-error-msg'" {
  run run_gate "fix/cli-error-msg"
  [ "$status" -eq 0 ]
}

@test "accepts a non-claude branch that merely contains the substring 'claude'" {
  # The gate must match the prefix, not the substring — a branch like
  # `feat/handle-claude-feedback` is legitimate.
  run run_gate "feat/handle-claude-feedback"
  [ "$status" -eq 0 ]
}

@test "accepts non-main branch named similarly to main" {
  # The gate must match exact `main`, not substrings — a branch like
  # `feat/main-page-redesign` is legitimate.
  run run_gate "feat/main-page-redesign"
  [ "$status" -eq 0 ]
}

@test "explicit empty-string arg is a caller bug, exits 2" {
  # Distinguishes 'no arg' (use git detection) from 'arg is empty
  # string'. The latter must surface a meaningful error rather than
  # silently falling through to git.
  run run_gate ""
  [ "$status" -eq 2 ]
  [[ "$output" == *"empty branch argument"* ]]
}
