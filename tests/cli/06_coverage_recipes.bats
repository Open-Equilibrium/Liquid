#!/usr/bin/env bats
#
# tests/cli/06_coverage_recipes.bats — smoke-test the two coverage-related
# justfile recipes added with item S1+S8 of the agent-discipline-r1 batch:
#
#   - `just coverage-check` — wraps `cargo tarpaulin --workspace
#     --skip-clean --fail-under 80 --out Stdout`. Chained into `just check`
#     after `deny-check`.
#   - `just check-ci`       — single recipe that runs the exact command
#     sequence `.github/workflows/ci.yml` runs for the Rust matrix job,
#     so an agent can reproduce CI locally with one verb.
#
# We don't execute `cargo tarpaulin` here (it is slow, requires a network
# install on first run, and is exercised in CI). We only assert that:
#
#   1. Both recipes exist in `just --list`.
#   2. `just check` lists `coverage-check` as a dependency after
#      `deny-check` (so the gate fires last, after the lighter checks).
#   3. `just check-ci` invokes the same three commands the CI workflow's
#      Rust job runs (`cargo fmt --all --check`, `cargo clippy --workspace
#      --all-targets --locked -- -D warnings`, `cargo test --workspace
#      --locked`).
#
# This is the project's enforcement that the `just`/CI parity does not
# silently drift — change the CI workflow and this test breaks.

# shellcheck shell=bash

ROOT="$BATS_TEST_DIRNAME/../.."

setup() {
  cd "$ROOT"
}

require_just() {
  if ! command -v just >/dev/null 2>&1; then
    skip "just not installed"
  fi
}

@test "just coverage-check recipe exists" {
  require_just
  run just --list
  [ "$status" -eq 0 ]
  [[ "$output" == *"coverage-check"* ]]
}

@test "just check-ci recipe exists" {
  require_just
  run just --list
  [ "$status" -eq 0 ]
  [[ "$output" == *"check-ci"* ]]
}

@test "just check chains coverage-check after deny-check" {
  require_just
  # `just --show <recipe>` prints the recipe body including the
  # dependency list on the header line: `check: lint test deny-check
  # coverage-check`. Asserting on this string instead of running the
  # recipe avoids paying the cargo build cost.
  run just --show check
  [ "$status" -eq 0 ]
  # Order matters: coverage-check must come AFTER deny-check so the
  # lighter clippy+test gates fire first.
  [[ "$output" == *"deny-check coverage-check"* ]]
}

@test "just coverage-check body matches the agreed cargo-tarpaulin invocation" {
  require_just
  run just --show coverage-check
  [ "$status" -eq 0 ]
  [[ "$output" == *"cargo tarpaulin"* ]]
  [[ "$output" == *"--workspace"* ]]
  [[ "$output" == *"--skip-clean"* ]]
  [[ "$output" == *"--fail-under 80"* ]]
  [[ "$output" == *"--out Stdout"* ]]
}

@test "just check-ci body matches the .github/workflows/ci.yml Rust job" {
  require_just
  run just --show check-ci
  [ "$status" -eq 0 ]
  # The three steps the Rust job runs, in order.
  [[ "$output" == *"cargo fmt --all --check"* ]]
  [[ "$output" == *"cargo clippy --workspace --all-targets --locked -- -D warnings"* ]]
  [[ "$output" == *"cargo test --workspace --locked"* ]]
}

@test "CONTRIBUTING.md lists cargo-tarpaulin as a prereq" {
  grep -q '`cargo-tarpaulin`' "$ROOT/CONTRIBUTING.md"
}
