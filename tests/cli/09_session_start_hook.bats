#!/usr/bin/env bats
#
# tests/cli/09_session_start_hook.bats — covers the
# `.claude/hooks/session-start.sh` greeting extension added with item
# F15 of the agent-discipline-r1 batch.
#
# Before the extension the hook printed a single line:
#
#     Liquid ready · branch=<branch> · cargo <version> · log=<path>
#
# After the extension, when run inside a git repo, it also prints a
# compact status block — branch, `git status --short`, and the 5
# most-recently-modified tracked files — capped at 30 lines so a
# resume turn does not waste Reads on stale context. Outside a git
# repo (or when git is absent), only the original greeting is
# emitted so we never break the start of a session.

# shellcheck shell=bash

ROOT="$BATS_TEST_DIRNAME/../.."
HOOK="$ROOT/.claude/hooks/session-start.sh"

setup() {
  cd "$ROOT"
}

@test "hook exists and is executable" {
  [ -x "$HOOK" ]
}

@test "hook parses with bash -n" {
  bash -n "$HOOK"
}

@test "hook prints the canonical greeting line" {
  run bash "$HOOK"
  [ "$status" -eq 0 ]
  [[ "$output" == *"Liquid ready"* ]]
  [[ "$output" == *"branch="* ]]
  [[ "$output" == *"log="* ]]
}

@test "hook prints a compact git status block when in a git repo" {
  run bash "$HOOK"
  [ "$status" -eq 0 ]
  # The block must include at least one of the markers the recently
  # added section emits — pick one that does not collide with the
  # canonical greeting line ("branch=" is shared).
  [[ "$output" == *"Recently modified"* || "$output" == *"recently modified"* ]]
}

@test "hook total chat output is capped at 30 lines" {
  run bash "$HOOK"
  [ "$status" -eq 0 ]
  line_count=$(printf '%s\n' "$output" | wc -l)
  [ "$line_count" -le 30 ] || {
    printf 'hook printed %d lines, exceeding the 30-line cap\n' "$line_count" >&2
    return 1
  }
}

@test "hook degrades cleanly outside a git repo" {
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  cp "$HOOK" "$tmp/session-start.sh"
  # Force a non-git CWD; the original greeting must still print and
  # the hook must exit 0.
  cd "$tmp"
  run bash "$tmp/session-start.sh"
  [ "$status" -eq 0 ]
  [[ "$output" == *"Liquid ready"* ]]
}
