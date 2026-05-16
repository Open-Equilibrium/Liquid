#!/usr/bin/env bats
#
# tests/cli/05_gh_job_log.bats — verifies `.claude/scripts/gh-job-log`
# behaviour on the network-free code paths:
#   - usage / arity / input-validation
#   - the awk parser's per-step bucketing (both `##[group]` raw mode
#     and `gh run view --log-failed` tab-separated mode)
#
# The actual API-fetch paths are not exercised — they require either
# a live `gh` CLI session or a valid `$GH_TOKEN`. CI hits those on
# real runs.

# shellcheck shell=bash

SCRIPT="$BATS_TEST_DIRNAME/../../.claude/scripts/gh-job-log"

setup() {
  TMP="$(mktemp -d -t liquid-ghlog-XXXXXX)"
  cd "$TMP"
  git init -q -b main
}

teardown() {
  cd /
  if [ -n "${TMP:-}" ] && [ -d "$TMP" ]; then
    rm -rf "$TMP"
  fi
}

@test "script exists and is executable" {
  [ -x "$SCRIPT" ]
}

@test "no-arg invocation prints usage and exits 2" {
  run bash "$SCRIPT"
  [ "$status" -eq 2 ]
  [[ "$output" == *"Usage"* ]]
}

@test "non-numeric run_id is rejected" {
  run bash "$SCRIPT" "../evil"
  [ "$status" -eq 2 ]
  [[ "$output" == *"positive integer"* ]]
}

@test "non-numeric job_id is rejected" {
  run bash "$SCRIPT" 12345 "../evil"
  [ "$status" -eq 2 ]
  [[ "$output" == *"positive integer"* ]]
}

# ── awk parser exercised in isolation ───────────────────────────────
#
# The script enforces arity at load time, so we can't cleanly source
# it. The parser is therefore vendored below. KEEP IN SYNC with the
# `print_tail_per_failed_step` body in `.claude/scripts/gh-job-log`.
# A divergence will silently let regressions ship.

_parser_one() {
  local fixture="$1"
  awk -F '\t' '
    function flush_bucket() {
      if (failed_in_bucket) {
        if (header != "") print header
        start = n - 50; if (start < 1) start = 1
        for (i = start; i <= n; i++) print buf[i]
        if (header != "") print "##[endgroup]"
      }
      delete buf; n = 0; failed_in_bucket = 0; header = ""
    }
    NR == 1 {
      if ($0 ~ /^##\[group\]/) { mode = "raw" } else if (NF >= 4) { mode = "gh" } else { mode = "raw" }
    }
    mode == "raw" && /^##\[group\]/   { flush_bucket(); in_group = 1; header = $0; next }
    mode == "raw" && /^##\[endgroup\]/{ flush_bucket(); in_group = 0; next }
    mode == "raw" {
      if (in_group) { n++; buf[n] = $0 }
      else          { print $0 }
      if ($0 ~ /##\[error\]|error\[E?[0-9]*\]|FAILED|failure|panicked/) failed_in_bucket = 1
      next
    }
    mode == "gh" {
      key = $2 "\t" $3
      if (key != prev_key && prev_key != "") flush_bucket()
      prev_key = key
      header = "##[step] " $2 " / " $3
      n++; buf[n] = $0
      if ($0 ~ /##\[error\]|error\[E?[0-9]*\]|FAILED|failure|panicked/) failed_in_bucket = 1
    }
    END { flush_bucket() }
  ' "$fixture" | tail -n 200
}

@test "raw-mode parser emits only failed groups" {
  cat > raw.log <<'EOF'
##[group]Step 1 — install deps
ok
ok
##[endgroup]
##[group]Step 2 — run tests
test_one ... ok
test_two ... FAILED
##[error]assertion failed
##[endgroup]
##[group]Step 3 — clean up
ok
##[endgroup]
EOF
  run _parser_one "$PWD/raw.log"
  [ "$status" -eq 0 ]
  [[ "$output" == *"Step 2"* ]]
  [[ "$output" == *"FAILED"* ]]
  [[ "$output" != *"Step 1"* ]]
  [[ "$output" != *"Step 3"* ]]
}

@test "gh tab-separated mode buckets by step and surfaces failed bucket" {
  # Hand-crafted gh-style log: 2 jobs × 2 steps, only one step has an
  # error marker.
  printf '%s\n' \
    "2026-01-01T00:00:00Z	Rust (ubuntu)	Install toolchain	ok" \
    "2026-01-01T00:00:01Z	Rust (ubuntu)	Install toolchain	ok" \
    "2026-01-01T00:00:02Z	Rust (ubuntu)	Tests	test_one ... ok" \
    "2026-01-01T00:00:03Z	Rust (ubuntu)	Tests	test_two ... FAILED" \
    "2026-01-01T00:00:04Z	Rust (ubuntu)	Tests	##[error]assertion failed" \
    > gh.log
  run _parser_one "$PWD/gh.log"
  [ "$status" -eq 0 ]
  [[ "$output" == *"Tests"* ]]
  [[ "$output" == *"FAILED"* ]]
  [[ "$output" != *"Install toolchain"* ]]
}

@test "raw-mode output is capped at 200 lines total" {
  # Build a failed group with 500 lines.
  {
    echo "##[group]Big step"
    for i in $(seq 1 499); do echo "noise-$i"; done
    echo "##[error]explosion"
    echo "##[endgroup]"
  } > big.log
  run _parser_one "$PWD/big.log"
  [ "$status" -eq 0 ]
  # Should be at most 200 lines (50 cap per group, plus headers,
  # plus tail -n 200 final cap).
  line_count=$(printf '%s\n' "$output" | wc -l)
  [ "$line_count" -le 200 ]
}
