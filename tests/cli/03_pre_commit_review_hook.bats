#!/usr/bin/env bats
#
# tests/cli/03_pre_commit_review_hook.bats — verifies the
# `.claude/hooks/pre-commit-review.sh` PreToolUse hook returns the
# right JSON envelope on every code path:
#
#   1. host-env opt-out (`LIQUID_SKIP_PRE_COMMIT_REVIEW=1`)
#   2. per-commit opt-out via `[skip-review]` token in the message
#   3. empty staged diff
#   4. non-empty staged diff (ask path)
#   5. snapshot file is created on the ask path
#   6. all paths emit valid JSON (jq -e parses)

# shellcheck shell=bash

HOOK="$BATS_TEST_DIRNAME/../../.claude/hooks/pre-commit-review.sh"

setup() {
  # Each test gets its own working copy so `git diff --staged` is
  # deterministic and unrelated to the actual repo state.
  TMP="$(mktemp -d -t liquid-hook-XXXXXX)"
  cd "$TMP"
  git init -q -b main
  git config user.email t@t.local
  git config user.name t
  # Disable any inherited commit-signing config — test environments
  # may have a forced sign-everything hook that we don't need here.
  git config commit.gpgsign false
  git config gpg.format ''
  echo "seed" > seed.txt
  git add seed.txt
  git commit -qm "seed" --no-gpg-sign
}

teardown() {
  cd /
  if [ -n "${TMP:-}" ] && [ -d "$TMP" ]; then
    rm -rf "$TMP"
  fi
}

@test "hook exists and is executable" {
  [ -x "$HOOK" ]
}

@test "host-env LIQUID_SKIP_PRE_COMMIT_REVIEW=1 returns continue:true" {
  run env LIQUID_SKIP_PRE_COMMIT_REVIEW=1 bash "$HOOK" <<<'{}'
  [ "$status" -eq 0 ]
  echo "$output" | jq -e '.continue == true' >/dev/null
}

@test "[skip-review] token in command bypasses the gate" {
  echo "new" > new.txt
  git add new.txt
  payload='{"tool_input":{"command":"git commit -m \"chore(ci): trivial [skip-review]\""}}'
  run bash -c "printf '%s' '$payload' | bash '$HOOK'"
  [ "$status" -eq 0 ]
  echo "$output" | jq -e '.continue == true' >/dev/null
}

@test "empty staged diff returns continue:true (silent allow)" {
  # Nothing staged.
  run bash "$HOOK" <<<'{}'
  [ "$status" -eq 0 ]
  echo "$output" | jq -e '.continue == true' >/dev/null
}

@test "non-empty staged diff returns hookSpecificOutput.permissionDecision = ask" {
  echo "real change" > change.txt
  git add change.txt
  payload='{"tool_input":{"command":"git commit -m \"feat(core): real\""}}'
  run bash -c "printf '%s' '$payload' | bash '$HOOK'"
  [ "$status" -eq 0 ]
  echo "$output" | jq -e '.hookSpecificOutput.hookEventName == "PreToolUse"' >/dev/null
  echo "$output" | jq -e '.hookSpecificOutput.permissionDecision == "ask"' >/dev/null
  # The reason text must point at the snapshot path and include the
  # opt-out instructions for the agent.
  echo "$output" | jq -e '.hookSpecificOutput.permissionDecisionReason | contains("pre-commit-")' >/dev/null
  echo "$output" | jq -e '.hookSpecificOutput.permissionDecisionReason | contains("[skip-review]")' >/dev/null
}

@test "ask path creates a snapshot file under .ai/artifacts/diffs/" {
  echo "snapshot test" > snap.txt
  git add snap.txt
  before=$(find .ai/artifacts/diffs -name 'pre-commit-*.diff' 2>/dev/null | wc -l)
  bash "$HOOK" <<<'{}' >/dev/null
  after=$(find .ai/artifacts/diffs -name 'pre-commit-*.diff' 2>/dev/null | wc -l)
  [ "$after" -gt "$before" ]
}

@test "files_changed in reason is correct (not off-by-one)" {
  # Stage exactly 1 file. The reason should say "1 files" not "2 files"
  # (off-by-one regression test for the original diff --stat parsing).
  echo "exactly-one" > one.txt
  git add one.txt
  run bash "$HOOK" <<<'{}'
  [ "$status" -eq 0 ]
  reason=$(echo "$output" | jq -r '.hookSpecificOutput.permissionDecisionReason')
  [[ "$reason" == *"1 files"* ]]
  [[ "$reason" != *"2 files"* ]]
}
