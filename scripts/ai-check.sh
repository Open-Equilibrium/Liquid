#!/usr/bin/env bash
# scripts/ai-check.sh — fast sanity check for the repo-local Claude Code
# configuration under `.claude/`.
#
# Run from the repo root (or via `just ai-check`). CI runs the same script,
# so anything caught here is caught before review. Output is intentionally
# verbose on failure and quiet on success.
#
# Checks performed:
#   1. `.claude/settings.json` parses with jq.
#   2. Every `.claude/hooks/*.sh` parses with `bash -n`.
#   3. Every `.claude/{skills,agents,rules,hooks}/<name>` path mentioned in
#      `CLAUDE.md` resolves to an actual file or directory on disk.
#   4. `.claude/hooks/filter-test-output.sh` produces non-empty output when
#      fed `tests/fixtures/noisy-cargo.log` on stdin.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

fail=0
say()  { printf '%s\n' "$*"; }
ok()   { printf '  ok   %s\n' "$*"; }
bad()  { printf '  FAIL %s\n' "$*" >&2; fail=1; }

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    say "ai-check: missing required tool: $1" >&2
    exit 2
  fi
}

require_cmd jq
require_cmd bash
require_cmd grep
require_cmd sort

# ── 1. settings.json ──────────────────────────────────────────────────────────
say "[1/4] .claude/settings.json"
if [ ! -f .claude/settings.json ]; then
  bad ".claude/settings.json is missing"
else
  if jq -e . .claude/settings.json >/dev/null 2>&1; then
    ok ".claude/settings.json parses"
  else
    bad ".claude/settings.json is not valid JSON"
  fi
fi

# ── 2. hook scripts ───────────────────────────────────────────────────────────
say "[2/4] .claude/hooks/*.sh"
hook_count=0
for hook in .claude/hooks/*.sh; do
  [ -f "$hook" ] || continue
  hook_count=$((hook_count + 1))
  # Capture stderr so a real syntax error surfaces in the FAIL line — the
  # previous `2>/dev/null` discarded the only useful diagnostic.
  if err=$(bash -n "$hook" 2>&1); then
    ok "$hook (bash -n)"
  else
    bad "$hook fails bash -n: $err"
  fi
done
if [ "$hook_count" -eq 0 ]; then
  bad "no scripts found under .claude/hooks/"
fi

# ── 3. references in CLAUDE.md ────────────────────────────────────────────────
say "[3/4] CLAUDE.md references vs disk"
if [ ! -f CLAUDE.md ]; then
  bad "CLAUDE.md is missing"
else
  # Extract every reference of the form `.claude/{skills,agents,rules,hooks}/<name>`
  # where <name> is a single path segment (letters, digits, dot, dash, underscore).
  # Dedup, then assert each path exists.
  refs=$(
    grep -Eo '\.claude/(skills|agents|rules|hooks)/[A-Za-z0-9._-]+' CLAUDE.md \
      | sort -u || true
  )
  if [ -z "$refs" ]; then
    bad "CLAUDE.md mentions no .claude/{skills,agents,rules,hooks}/<name> paths"
  else
    while IFS= read -r ref; do
      [ -z "$ref" ] && continue
      if [ -e "$ref" ]; then
        ok "$ref"
      else
        bad "$ref referenced in CLAUDE.md but missing on disk"
      fi
    done <<<"$refs"
  fi
fi

# ── 4. filter-test-output.sh smoke test ───────────────────────────────────────
say "[4/4] .claude/hooks/filter-test-output.sh smoke test"
fixture=tests/fixtures/noisy-cargo.log
if [ ! -f .claude/hooks/filter-test-output.sh ]; then
  bad ".claude/hooks/filter-test-output.sh missing"
elif [ ! -f "$fixture" ]; then
  bad "$fixture missing — the smoke test needs a known-noisy input"
else
  tmpdir=$(mktemp -d)
  trap 'rm -rf "$tmpdir"' EXIT
  # Run from a scratch CWD so we do not pollute the repo's .ai/ tree.
  # Capture the repo root explicitly rather than relying on $OLDPWD being
  # set correctly inside the subshell.
  repo_root=$(pwd)
  out=$(
    cd "$tmpdir"
    "$repo_root/.claude/hooks/filter-test-output.sh" <"$repo_root/$fixture"
  )
  if [ -n "$out" ] && printf '%s' "$out" | grep -qiE 'fail|error|panicked'; then
    ok "filter-test-output.sh emits failure-oriented summary"
  else
    bad "filter-test-output.sh produced no failure-oriented lines for $fixture"
  fi
fi

if [ "$fail" -ne 0 ]; then
  say ""
  say "ai-check: one or more checks failed" >&2
  exit 1
fi

say ""
say "ai-check: all checks passed"
