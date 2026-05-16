#!/usr/bin/env bats
#
# tests/cli/04_changelog_gate.bats — verifies the CHANGELOG-discipline
# commit-msg gate at `.lefthook/commit-msg/check-changelog.sh`.
#
# Policy (CONTRIBUTING.md "Documentation as part of the change"):
#   - feat / fix / refactor / perf commits MUST modify CHANGELOG.md
#     (or include a `[no-changelog]` trailer).
#   - docs / test / chore(ci|claude|deps|ai|gh|tooling) are exempt.

# shellcheck shell=bash

GATE="$BATS_TEST_DIRNAME/../../.lefthook/commit-msg/check-changelog.sh"

setup() {
  TMP="$(mktemp -d -t liquid-changelog-XXXXXX)"
  cd "$TMP"
  git init -q -b main
  git config user.email t@t.local
  git config user.name t
  git config commit.gpgsign false
  git config gpg.format ''
  echo "seed" > seed.txt
  git add seed.txt
  git commit -qm "seed" --no-gpg-sign
  touch CHANGELOG.md
}

teardown() {
  cd /
  if [ -n "${TMP:-}" ] && [ -d "$TMP" ]; then
    rm -rf "$TMP"
  fi
}

# Helper: write the message to a temp file and stage some files; then
# run the gate.
run_gate_with_subject_and_staged() {
  local subject="$1"; shift
  local msg_file
  msg_file=$(mktemp)
  echo "$subject" > "$msg_file"
  if [ "$#" -gt 0 ]; then
    printf '\n%s\n' "$@" >> "$msg_file"
  fi
  bash "$GATE" "$msg_file"
}

@test "gate exists and is executable" {
  [ -x "$GATE" ]
}

@test "exits 2 on missing message file" {
  run bash "$GATE" /nonexistent
  [ "$status" -eq 2 ]
}

@test "feat without CHANGELOG is rejected" {
  echo "x" > new.txt && git add new.txt
  run run_gate_with_subject_and_staged "feat(core): add thing"
  [ "$status" -eq 1 ]
  [[ "$output" == *"CHANGELOG"* ]]
}

@test "fix without CHANGELOG is rejected" {
  echo "y" > fix.txt && git add fix.txt
  run run_gate_with_subject_and_staged "fix(auth): correct token expiry"
  [ "$status" -eq 1 ]
}

@test "refactor without CHANGELOG is rejected" {
  echo "z" > refactor.txt && git add refactor.txt
  run run_gate_with_subject_and_staged "refactor(vcs): extract helper"
  [ "$status" -eq 1 ]
}

@test "feat WITH CHANGELOG passes" {
  echo "x" > new.txt && git add new.txt CHANGELOG.md
  run run_gate_with_subject_and_staged "feat(core): add thing"
  [ "$status" -eq 0 ]
}

@test "feat with [no-changelog] trailer passes" {
  echo "x" > new.txt && git add new.txt
  run run_gate_with_subject_and_staged "feat(core): add thing" "Refactor of internal helper." "[no-changelog]"
  [ "$status" -eq 0 ]
}

@test "docs is exempt" {
  echo "x" > new.txt && git add new.txt
  run run_gate_with_subject_and_staged "docs(readme): tweak"
  [ "$status" -eq 0 ]
}

@test "test is exempt" {
  echo "x" > new.txt && git add new.txt
  run run_gate_with_subject_and_staged "test(cli): add bats cases"
  [ "$status" -eq 0 ]
}

@test "chore(ci) is exempt" {
  echo "x" > new.txt && git add new.txt
  run run_gate_with_subject_and_staged "chore(ci): adjust workflow"
  [ "$status" -eq 0 ]
}

@test "chore(claude) is exempt" {
  echo "x" > new.txt && git add new.txt
  run run_gate_with_subject_and_staged "chore(claude): tweak hook"
  [ "$status" -eq 0 ]
}

@test "chore(deps) is exempt" {
  echo "x" > new.txt && git add new.txt
  run run_gate_with_subject_and_staged "chore(deps): bump serde"
  [ "$status" -eq 0 ]
}

@test "chore(core) is NOT exempt — chore with non-tooling scope still needs CHANGELOG" {
  echo "x" > new.txt && git add new.txt
  run run_gate_with_subject_and_staged "chore(core): rename internal type"
  [ "$status" -eq 1 ]
}

@test "non-Conventional subject is left to the sibling check, exits 0" {
  echo "x" > new.txt && git add new.txt
  run run_gate_with_subject_and_staged "Some random subject"
  [ "$status" -eq 0 ]
}
