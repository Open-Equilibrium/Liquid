#!/usr/bin/env bash
set -euo pipefail

# Token-efficient helper for filtering noisy Rust and Flutter/Dart output.
#
# Manual usage:
#   cargo test 2>&1 | .claude/hooks/filter-test-output.sh
#   flutter test 2>&1 | .claude/hooks/filter-test-output.sh
#   flutter analyze 2>&1 | .claude/hooks/filter-test-output.sh
#   bats tests/cli/ 2>&1 | .claude/hooks/filter-test-output.sh
#
# Stores raw stdin and emits a compact failure-oriented summary on stdout.

mkdir -p .ai/artifacts/logs

ts="$(date -u +%Y%m%dT%H%M%SZ)"
raw=".ai/artifacts/logs/raw-${ts}.log"
summary=".ai/artifacts/logs/summary-${ts}.log"

cat > "$raw" || true

{
  echo "Raw log: $raw"
  echo
  echo "Failure-oriented excerpt:"
  grep -Ein \
    "error\[E[0-9]+\]|error:|warning:|fail|failed|failure|panicked|panic|thread '.*' panicked|assertion|expected|actual|left:|right:|stack trace|exception|flutter|dart|analyzer|undefined|not found|type .* is not assignable|NoSuchMethodError|LateInitializationError|RenderFlex|overflowed|golden|diff|timeout|timed out|not ok|✗|×" \
    "$raw" | head -n 160 || true
  echo
  echo "Tail:"
  tail -n 100 "$raw" || true
} > "$summary"

cat "$summary"
