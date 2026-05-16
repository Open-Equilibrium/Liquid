#!/usr/bin/env bash
# .lefthook/commit-msg/check-changelog.sh — CHANGELOG-discipline gate.
#
# Called by lefthook's `commit-msg` hook with the path to the
# COMMIT_EDITMSG file as $1.
#
# Policy (CLAUDE.md "Step 7 — Documentation update" + CONTRIBUTING.md
# "Documentation as part of the change"):
#
#   - Any commit whose subject line is `feat(...)`, `fix(...)`,
#     `refactor(...)`, or `perf(...)` MUST either:
#       (a) include a modification to `CHANGELOG.md` in the same
#           commit (`git diff --cached --name-only`), or
#       (b) carry a `[no-changelog]` trailer in the message body
#           (followed by a one-line justification).
#
#   - `docs(...)`, `test(...)`, and `chore(ci|claude|deps|…)` commits
#     are exempt because they do not change user-visible behaviour;
#     `chore(<other>)` is treated like `feat` / `fix` (it likely
#     does change behaviour even if the scope is unconventional).
#
# The gate fires on the local `commit-msg` hook so a misformed commit
# is caught BEFORE it lands. CI cannot enforce this retroactively
# without breaking history.

set -euo pipefail

msg_file="${1:?usage: $0 <COMMIT_EDITMSG>}"
[ -f "$msg_file" ] || {
  printf 'check-changelog: commit message file not found: %s\n' "$msg_file" >&2
  exit 2
}

subject=$(head -1 "$msg_file")
body=$(tail -n +2 "$msg_file")

# Pull the type from the subject. If the message does not follow
# Conventional Commits, the sibling `check-conventional.sh` hook will
# reject it; here we just no-op.
type_scope=$(printf '%s\n' "$subject" | grep -oE '^[a-z]+(\([a-z0-9_-]+\))?' || true)
type=$(printf '%s\n' "$type_scope" | cut -d'(' -f1)
scope=$(printf '%s\n' "$type_scope" | sed -nE 's/^[a-z]+\(([a-z0-9_-]+)\)$/\1/p')

if [ -z "$type" ]; then
  # Not Conventional — let check-conventional handle it.
  exit 0
fi

# Exempt types (no user-visible behaviour change).
case "$type" in
  docs|test)
    exit 0
    ;;
  chore)
    case "$scope" in
      ci|claude|deps|ai|gh|tooling)
        exit 0
        ;;
    esac
    ;;
esac

# Trailer opt-out: `[no-changelog]` anywhere in the body. Use
# sparingly — reviewers should sanity-check the reason.
if printf '%s' "$body" | grep -qE '\[no-changelog\]'; then
  exit 0
fi

# Was CHANGELOG.md staged in this commit?
# `git diff --cached --name-only` lists files in the index that will
# be in the commit. Repository root is wherever the hook runs from
# (lefthook starts hooks at the repo root).
if git diff --cached --name-only 2>/dev/null | grep -qx 'CHANGELOG.md'; then
  exit 0
fi

cat >&2 <<EOF
✗ CHANGELOG discipline: this commit changes user-visible behaviour
  (\`${type_scope}\`) but does not modify CHANGELOG.md.

  Add an entry under \`## [Unreleased]\` and re-stage CHANGELOG.md
  before committing.

  Exempt types: docs, test, chore(ci|claude|deps|ai|gh|tooling).

  If a CHANGELOG entry is genuinely unnecessary (e.g. fixing a
  pre-merged bug that never reached a release), add a
  \`[no-changelog]\` trailer to the commit body explaining why.

  Got subject: ${subject}
EOF
exit 1
