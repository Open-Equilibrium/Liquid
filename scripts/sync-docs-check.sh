#!/usr/bin/env bash
# scripts/sync-docs-check.sh — report-only sync-docs gate.
#
# Compares milestone status across the four documents that record it:
#   - README.md         (Status table)
#   - TASKS.md          (Done section vs Active section)
#   - CHANGELOG.md      (## [Unreleased] block)
#   - IMPLEMENTATION_PLAN.md §5 (per-milestone checkboxes / status)
#
# Reports mismatches. Never edits files. Exits non-zero if a mismatch is
# found so CI can fail loud while keeping the doc set the human's call.
#
# Cross-checks performed:
#   1. Each Mn marked ✅ Done in README.md has an entry in CHANGELOG.md
#      under [Unreleased] or in a previous tagged section.
#   2. Each Mn marked ✅ Done in README.md has matching evidence in
#      IMPLEMENTATION_PLAN.md §5 (no remaining unchecked top-level
#      bullets for that milestone).
#   3. Each TASK-NNN referenced in IMPLEMENTATION_PLAN.md or CHANGELOG.md
#      resolves to a heading in TASKS.md.
#
# These mirror checks (1), (2), and (3) of `.claude/skills/sync-docs/SKILL.md`.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

fail=0
note() { printf 'sync-docs: %s\n' "$*"; }
bad()  { printf 'sync-docs: FAIL  %s\n' "$*" >&2; fail=1; }

require() {
  if [ ! -f "$1" ]; then
    bad "missing $1"
    return 1
  fi
}

require README.md
require TASKS.md
require CHANGELOG.md
require IMPLEMENTATION_PLAN.md

# ── Parse README status table ────────────────────────────────────────────────
# Lines look like: | **M3** Auth + permissions | ... | ✅ Done |
# We capture the milestone id (M<digits>(.<digits>)?) and whether it is Done.
readme_done=()
while IFS= read -r line; do
  if [[ "$line" =~ \*\*(M[0-9]+(\.[0-9]+)?)\*\* ]]; then
    id="${BASH_REMATCH[1]}"
    if echo "$line" | grep -q '✅'; then
      readme_done+=("$id")
    fi
  fi
done < README.md

# ── Cross-check: every README ✅ has a CHANGELOG entry ───────────────────────
# Milestone IDs may contain dots (M6.5). Escape every dot before stuffing
# the id into an ERE so the dot does not act as a wildcard (which would
# silently match M6X5 as well as M6.5).
escape_ere() { printf '%s' "$1" | sed 's/\./\\./g'; }

for m in "${readme_done[@]:-}"; do
  [ -z "$m" ] && continue
  m_esc=$(escape_ere "$m")
  if grep -Eq "\b$m_esc\b" CHANGELOG.md; then
    : # ok
  else
    bad "$m marked ✅ Done in README.md but no mention in CHANGELOG.md"
  fi
done

# ── Cross-check: every README ✅ has §5 evidence in IMPLEMENTATION_PLAN ──────
for m in "${readme_done[@]:-}"; do
  [ -z "$m" ] && continue
  m_esc=$(escape_ere "$m")
  m_num_esc=$(escape_ere "${m#M}")
  # Accept either the legacy form "### 5.N Milestone N —" or the
  # versioned form "### 5.N Milestone M6.5 —". Phase-2 milestones
  # (M8 / M9 / M10 / …) live under §6 rather than §5, so accept either
  # section prefix.
  if grep -Eq "^### [56]\.[0-9]+ Milestone ($m_esc|$m_num_esc) " IMPLEMENTATION_PLAN.md; then
    : # ok
  else
    bad "$m marked ✅ Done in README.md but IMPLEMENTATION_PLAN.md §5/§6 has no matching heading"
  fi
done

# ── Cross-check: TASK-NNN references resolve ─────────────────────────────────
tasks_known=$(grep -Eo '\[TASK-[0-9]+\]|\bTASK-[0-9]+\b' TASKS.md \
  | grep -Eo 'TASK-[0-9]+' | sort -u || true)
tasks_referenced=$(
  grep -Eho '\bTASK-[0-9]+\b' IMPLEMENTATION_PLAN.md CHANGELOG.md \
    docs/adr/*.md 2>/dev/null | sort -u || true
)
while IFS= read -r ref; do
  [ -z "$ref" ] && continue
  if printf '%s\n' "$tasks_known" | grep -Fxq "$ref"; then
    : # ok
  else
    bad "$ref referenced in plan/changelog/ADR but not defined in TASKS.md"
  fi
done <<<"$tasks_referenced"

# ── Report ───────────────────────────────────────────────────────────────────
if [ "$fail" -eq 0 ]; then
  note "doc set is internally consistent (README ↔ CHANGELOG ↔ plan ↔ TASKS)"
  exit 0
fi
exit 1
