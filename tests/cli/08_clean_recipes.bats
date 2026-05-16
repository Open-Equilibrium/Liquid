#!/usr/bin/env bats
#
# tests/cli/08_clean_recipes.bats — covers the `just clean` family
# added with item S9+S10 of the agent-discipline-r1 batch.
#
# The walkthrough examples (`liquid-vcs::m2_walkthrough` and
# `liquid-permissions::m3_walkthrough`) write self-asserting
# demonstration state under `/tmp/liquid-m{2,3}-walkthrough/`. Re-runs
# expect a fresh directory each time and the on-disk artifacts are
# kept after the run for human inspection — which is fine for the
# happy path but leaves stale state when an agent moves on to a
# different milestone. `just clean-walkthroughs` is the explicit verb
# that removes it.
#
# Tests are filesystem smoke-tests; they neither execute the
# walkthroughs nor depend on cargo.

# shellcheck shell=bash

ROOT="$BATS_TEST_DIRNAME/../.."

require_just() {
  if ! command -v just >/dev/null 2>&1; then
    skip "just not installed"
  fi
}

setup() {
  cd "$ROOT"
}

@test "just clean recipe exists" {
  require_just
  run just --list
  [ "$status" -eq 0 ]
  [[ "$output" == *" clean "* || "$output" == *" clean$"* || "$output" == *" clean
"* ]] || [[ "$output" == *$'\n''    clean '* ]]
  # Cheaper assertion: --show fails cleanly on a missing recipe.
  run just --show clean
  [ "$status" -eq 0 ]
}

@test "just clean-walkthroughs recipe exists" {
  require_just
  run just --show clean-walkthroughs
  [ "$status" -eq 0 ]
}

@test "just clean depends on clean-walkthroughs" {
  require_just
  run just --show clean
  [ "$status" -eq 0 ]
  [[ "$output" == *"clean-walkthroughs"* ]]
}

@test "just clean-walkthroughs body removes <temp_dir>/liquid-m*-walkthrough" {
  require_just
  run just --show clean-walkthroughs
  [ "$status" -eq 0 ]
  # Must reference both the rm command, the temp-dir resolution
  # (`${TMPDIR:-/tmp}` to honour macOS's `/private/tmp`), and the
  # wildcarded target so adding a new milestone walkthrough does
  # not require a recipe edit.
  [[ "$output" == *"rm -rf"* ]]
  [[ "$output" == *'${TMPDIR:-/tmp}'* ]]
  [[ "$output" == *"liquid-m"* ]]
  [[ "$output" == *"-walkthrough"* ]]
}

# Resolve the temp dir the way the walkthroughs (and the recipe) do:
# `${TMPDIR:-/tmp}`. macOS sets TMPDIR; Linux normally does not.
tmpdir() {
  printf '%s\n' "${TMPDIR:-/tmp}"
}

@test "just clean-walkthroughs is idempotent on a clean tree" {
  require_just
  # Pre-state: make sure the target dirs do NOT exist (mirrors a fresh
  # container). Run twice — the second invocation must still exit 0.
  td="$(tmpdir)"
  rm -rf "$td/liquid-m2-walkthrough" "$td/liquid-m3-walkthrough"
  run just clean-walkthroughs
  [ "$status" -eq 0 ]
  run just clean-walkthroughs
  [ "$status" -eq 0 ]
}

@test "just clean-walkthroughs removes a seeded artifact dir" {
  require_just
  td="$(tmpdir)"
  mkdir -p "$td/liquid-m2-walkthrough/files"
  mkdir -p "$td/liquid-m3-walkthrough/auth"
  echo seeded > "$td/liquid-m2-walkthrough/files/x"
  echo seeded > "$td/liquid-m3-walkthrough/auth/y"

  run just clean-walkthroughs
  [ "$status" -eq 0 ]
  [ ! -e "$td/liquid-m2-walkthrough" ]
  [ ! -e "$td/liquid-m3-walkthrough" ]
}

@test "m2_walkthrough main delegates to a named helper" {
  # Refactor target: main() must call a named async helper (not
  # contain the whole walkthrough body). Asserting on the source
  # keeps the refactor honest.
  grep -qE 'fn (run_walkthrough|walkthrough)\s*\(' \
    "$ROOT/core/liquid-vcs/examples/m2_walkthrough.rs"
}

@test "m3_walkthrough main delegates to a named helper" {
  grep -qE 'fn (run_walkthrough|walkthrough)\s*\(' \
    "$ROOT/core/liquid-permissions/examples/m3_walkthrough.rs"
}

@test "walkthroughs no longer carry #[allow(clippy::too_many_lines)]" {
  # Match the attribute form only — docstrings that mention the
  # lint name in passing must not trigger.
  ! grep -nE '#\[allow\(clippy::too_many_lines\)\]' \
    "$ROOT/core/liquid-vcs/examples/m2_walkthrough.rs" \
    "$ROOT/core/liquid-permissions/examples/m3_walkthrough.rs"
}
