#!/usr/bin/env bats
#
# tests/cli/02_bump_version.bats — verifies `scripts/bump-version.sh`
# (wrapped by `just bump-version <new>`) atomically updates every
# version literal in `core/Cargo.toml` that tracks the workspace
# release version. Per `core/Cargo.toml` workspace conventions:
#
#   - `[workspace.package].version` is the canonical workspace
#     release version (consumed by every crate via `version.workspace
#     = true`).
#   - `[workspace.dependencies].liquid-*.version` is the literal
#     duplicated next to each `path = "..."` entry. cargo-deny (and
#     crates.io's path-publish requirement) treats path-only deps as
#     wildcards, so the literal must be present AND must match
#     `[workspace.package].version` at all times.
#
# A LIQUID_VERSION source-of-truth means one command — `just
# bump-version <new>` — rewrites all 8 occurrences (1 workspace +
# 7 path-deps) atomically; nothing can drift.

# shellcheck shell=bash

BUMP="$BATS_TEST_DIRNAME/../../scripts/bump-version.sh"

setup() {
  # Copy the real core/Cargo.toml into a scratch dir so the test
  # never mutates the actual workspace manifest.
  TMP="$(mktemp -d -t liquid-bump-XXXXXX)"
  cp "$BATS_TEST_DIRNAME/../../core/Cargo.toml" "$TMP/Cargo.toml"
  export BUMP_MANIFEST="$TMP/Cargo.toml"
}

teardown() {
  if [ -n "${TMP:-}" ] && [ -d "$TMP" ]; then
    rm -rf "$TMP"
  fi
}

run_bump() {
  bash "$BUMP" --manifest "$BUMP_MANIFEST" "$@"
}

@test "script exists and is executable" {
  [ -x "$BUMP" ]
}

@test "rejects missing version argument" {
  run run_bump
  [ "$status" -ne 0 ]
  [[ "$output" == *"version"* ]]
}

@test "rejects malformed version" {
  run run_bump "not.a.version"
  [ "$status" -ne 0 ]
  [[ "$output" == *"semver"* || "$output" == *"version"* ]]
}

@test "accepts SemVer 0.2.0 and rewrites every version literal" {
  run run_bump "0.2.0"
  [ "$status" -eq 0 ]
  # workspace.package.version
  grep -qE '^version = "0\.2\.0"' "$BUMP_MANIFEST"
  # All 7 workspace path-deps must also have version = "0.2.0"
  count=$(grep -cE '\{ *path = "liquid-[a-z][a-z-]*", *version = "0\.2\.0" *\}' "$BUMP_MANIFEST")
  [ "$count" -eq 7 ]
  # The OLD version literal must be gone entirely.
  ! grep -qE 'version = "0\.1\.0"' "$BUMP_MANIFEST"
}

@test "accepts SemVer pre-release 0.2.0-pre.M4" {
  run run_bump "0.2.0-pre.M4"
  [ "$status" -eq 0 ]
  grep -qE '^version = "0\.2\.0-pre\.M4"' "$BUMP_MANIFEST"
  count=$(grep -cE '\{ *path = "liquid-[a-z][a-z-]*", *version = "0\.2\.0-pre\.M4" *\}' "$BUMP_MANIFEST")
  [ "$count" -eq 7 ]
}

@test "is idempotent — same version twice is a no-op exit 0" {
  run_bump "0.3.0"
  run run_bump "0.3.0"
  [ "$status" -eq 0 ]
  grep -qE '^version = "0\.3\.0"' "$BUMP_MANIFEST"
}

@test "leaves rust-version untouched" {
  before=$(grep -E '^rust-version *=' "$BUMP_MANIFEST")
  run_bump "0.5.0"
  after=$(grep -E '^rust-version *=' "$BUMP_MANIFEST")
  [ "$before" = "$after" ]
}

@test "leaves third-party dep versions untouched" {
  # Pick a few external deps whose version literals must NOT change
  # when LIQUID_VERSION moves.
  before_serde=$(grep -E '^serde *=' "$BUMP_MANIFEST")
  before_tokio=$(grep -E '^tokio *=' "$BUMP_MANIFEST")
  run_bump "0.9.0"
  after_serde=$(grep -E '^serde *=' "$BUMP_MANIFEST")
  after_tokio=$(grep -E '^tokio *=' "$BUMP_MANIFEST")
  [ "$before_serde" = "$after_serde" ]
  [ "$before_tokio" = "$after_tokio" ]
}
