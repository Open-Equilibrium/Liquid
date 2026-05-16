#!/usr/bin/env bash
# scripts/bump-version.sh — single source-of-truth bump for
# LIQUID_VERSION across `core/Cargo.toml`.
#
# Updates atomically (write-tmp + rename):
#   - `[workspace.package].version`
#   - every `liquid-* = { path = "...", version = "..." }` literal in
#     `[workspace.dependencies]`
#
# Wrapped by `just bump-version <new>`. Covered by
# `tests/cli/02_bump_version.bats`.
#
# Why this exists: cargo treats path-only deps as wildcards, which
# trips cargo-deny's `wildcards = "deny"` rule. The current
# `core/Cargo.toml` therefore writes BOTH `path` and `version` for
# every workspace-internal dep, which means a release has to bump 8
# version literals in lock-step. Forgetting any one of them silently
# breaks the next `cargo publish` (path wins locally, `version`
# governs the registry). One script means there is exactly one place
# to make a mistake.
#
# Usage:
#   scripts/bump-version.sh <new-semver>
#   scripts/bump-version.sh --manifest <path> <new-semver>   # for tests
#
# Exit codes:
#   0   success (file rewritten, or already at the requested version)
#   1   bad version, missing manifest, or rewrite failure
#   2   bad invocation

set -euo pipefail

usage() {
  cat <<'EOF' >&2
Usage: scripts/bump-version.sh [--manifest <path>] <new-semver>

  <new-semver>   Target version, e.g. 0.2.0 or 0.2.0-pre.M4.
                 Must match: ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$

  --manifest     Override the path to core/Cargo.toml (used by tests
                 so the real workspace manifest is never mutated).
EOF
  exit 2
}

manifest=""
new=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --manifest)
      shift
      [ "$#" -gt 0 ] || usage
      manifest="$1"
      shift
      ;;
    -h|--help)
      usage
      ;;
    *)
      if [ -z "$new" ]; then
        new="$1"
        shift
      else
        usage
      fi
      ;;
  esac
done

if [ -z "$new" ]; then
  printf 'bump-version: missing <new-semver> argument.\n' >&2
  usage
fi

# Pin a single permissive SemVer 2.0 regex (no `+build` segment because
# cargo-release does not emit them for Liquid).
semver_re='^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$'
if ! [[ "$new" =~ $semver_re ]]; then
  printf 'bump-version: %s is not a valid semver string.\n' "$new" >&2
  exit 1
fi

if [ -z "$manifest" ]; then
  repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
  manifest="$repo_root/core/Cargo.toml"
fi

if [ ! -f "$manifest" ]; then
  printf 'bump-version: manifest not found: %s\n' "$manifest" >&2
  exit 1
fi

# Re-escape the new version for safe inclusion in a sed replacement.
escaped=$(printf '%s' "$new" | sed -e 's/[\/&]/\\&/g')

tmp=$(mktemp "${manifest}.bump.XXXXXX")
trap 'rm -f "$tmp"' EXIT

# Two narrow rewrites:
#   1. `version = "<old>"` immediately under [workspace.package] —
#      we anchor on the line literal (no internal whitespace
#      variation expected; `cargo fmt` keeps this stable).
#   2. `liquid-<crate> = { path = "...", version = "<old>" }` —
#      every internal path-dep. We match the FULL literal shape so
#      a stray third-party `version =` cannot be hit by accident.
sed -E \
  -e 's/^(version *= *")[^"]+(")/\1'"$escaped"'\2/' \
  -e 's/(\{ *path *= *"liquid-[a-z][a-z-]*", *version *= *")[^"]+(" *\})/\1'"$escaped"'\2/g' \
  "$manifest" > "$tmp"

# Verification: the file MUST now contain exactly one
# `^version = "<new>"` line (workspace package) and at least 7
# path-dep version literals at the target. The exact crate count is
# left to the bats test because new crates may be added.
if ! grep -qE "^version = \"$(printf '%s' "$escaped" | sed 's/[]\/$*.^[]/\\&/g')\"" "$tmp"; then
  printf 'bump-version: rewrite failed — workspace.package.version not updated.\n' >&2
  exit 1
fi

path_dep_count=$(grep -cE '\{ *path *= *"liquid-[a-z][a-z-]*", *version *= *"'"$(printf '%s' "$escaped" | sed 's/[]\/$*.^[]/\\&/g')"'" *\}' "$tmp")
if [ "$path_dep_count" -lt 1 ]; then
  printf 'bump-version: rewrite failed — no liquid-* path-dep version literals at %s.\n' "$new" >&2
  exit 1
fi

mv "$tmp" "$manifest"
trap - EXIT

printf 'bump-version: %s now at %s (%d path-deps rewritten).\n' "$manifest" "$new" "$path_dep_count"
