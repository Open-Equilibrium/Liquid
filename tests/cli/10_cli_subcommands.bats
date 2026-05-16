#!/usr/bin/env bats
#
# tests/cli/10_cli_subcommands.bats — focused per-subcommand tests
# for the M6.5 `liquid` CLI surface (TASK-008). Complements
# `00_mvp_slice.bats` (end-to-end happy path) with edge cases and
# negative paths every subcommand should handle.

# shellcheck shell=bash

setup() {
  export LIQUID_HOME="$(mktemp -d -t liquid-cli-XXXXXX)"
  if ! command -v liquid >/dev/null 2>&1; then
    local target_bin="$BATS_TEST_DIRNAME/../../core/target/debug/liquid"
    if [ -x "$target_bin" ]; then
      export PATH="$(dirname "$target_bin"):$PATH"
    fi
  fi
  if ! command -v jq >/dev/null 2>&1; then
    skip "jq not installed"
  fi
}

teardown() {
  if [ -n "${LIQUID_HOME:-}" ] && [ -d "$LIQUID_HOME" ]; then
    rm -rf "$LIQUID_HOME"
  fi
}

@test "version flag prints a non-empty version" {
  run liquid --version
  [ "$status" -eq 0 ]
  [[ "$output" =~ ^liquid[[:space:]]+[0-9]+\.[0-9]+\.[0-9]+ ]]
}

@test "no args prints help and exits non-zero (clap convention)" {
  run liquid
  [ "$status" -ne 0 ]
  [[ "$output" == *"Usage:"* ]] || [[ "$output" == *"usage:"* ]]
}

@test "workspace create bootstraps secret + token files on first run" {
  run liquid --format json workspace create demo
  [ "$status" -eq 0 ]
  [ -f "$LIQUID_HOME/secret" ]
  [ -f "$LIQUID_HOME/token" ]
  # Secret must be ≥ 16 bytes (LocalIdentityProvider requirement).
  size=$(wc -c < "$LIQUID_HOME/secret")
  [ "$size" -ge 16 ]
  # Token format: u:<uuid>.<expires>.<hmac> (bootstrap user, not agent).
  [[ "$(cat $LIQUID_HOME/token)" =~ ^u:[0-9a-f-]+\.[0-9]+\.[0-9a-f]+$ ]]
}

@test "workspace create persists registry across invocations" {
  ws1=$(liquid --format json workspace create alpha | jq -r '.data.workspace_id')
  ws2=$(liquid --format json workspace create beta  | jq -r '.data.workspace_id')
  [ -n "$ws1" ] && [ "$ws1" != "null" ]
  [ -n "$ws2" ] && [ "$ws2" != "null" ]
  [ "$ws1" != "$ws2" ]
  # registry.toml must record both.
  [ -f "$LIQUID_HOME/registry/workspaces.toml" ]
  grep -q "$ws1" "$LIQUID_HOME/registry/workspaces.toml"
  grep -q "$ws2" "$LIQUID_HOME/registry/workspaces.toml"
}

@test "auth token prints the active token" {
  liquid --format json workspace create demo >/dev/null
  run liquid --format json auth token
  [ "$status" -eq 0 ]
  token=$(echo "$output" | jq -r '.data.token')
  [ -n "$token" ] && [ "$token" != "null" ]
  # Should match what was written to disk.
  [ "$token" = "$(cat $LIQUID_HOME/token)" ]
}

@test "auth token errors when no token is on disk" {
  # No `workspace create` first ⇒ no bootstrap ⇒ no token file.
  run liquid --format json auth token
  [ "$status" -eq 2 ]   # InvalidInput → EX_USAGE
  [ "$(echo "$output" | jq -r '.ok')" = "false" ]
}

@test "page write rejects invalid workspace uuid with InvalidInput" {
  liquid --format json workspace create demo >/dev/null
  run liquid --format json page write /pages/x --workspace "not-a-uuid" --data '{"v":1}'
  [ "$status" -eq 2 ]
  [ "$(echo "$output" | jq -r '.ok')" = "false" ]
  [[ "$(echo "$output" | jq -r '.error')" == *"workspace id not a uuid"* ]]
}

@test "page write requires either --data or --file" {
  ws=$(liquid --format json workspace create demo | jq -r '.data.workspace_id')
  run liquid --format json page write /pages/x --workspace "$ws"
  [ "$status" -eq 2 ]
  [ "$(echo "$output" | jq -r '.ok')" = "false" ]
}

@test "page write --file reads bytes from the named file" {
  ws=$(liquid --format json workspace create demo | jq -r '.data.workspace_id')
  member_token=$(liquid --format json auth provision-agent w \
    --workspace "$ws" --role WorkspaceMember | jq -r '.data.token')
  export LIQUID_TOKEN="$member_token"
  payload_file="$LIQUID_HOME/payload.json"
  echo '{"k":"v"}' > "$payload_file"
  run liquid --format json page write /pages/notes --workspace "$ws" --file "$payload_file"
  [ "$status" -eq 0 ]
  run liquid --format json page read /pages/notes --workspace "$ws"
  [ "$status" -eq 0 ]
  [ "$(echo "$output" | jq -r '.data.k')" = "v" ]
}

@test "page read returns NotFound (exit 1) for an unknown path" {
  ws=$(liquid --format json workspace create demo | jq -r '.data.workspace_id')
  run liquid --format json page read /pages/never-written --workspace "$ws"
  [ "$status" -eq 1 ]
  [ "$(echo "$output" | jq -r '.ok')" = "false" ]
  [[ "$(echo "$output" | jq -r '.error')" =~ [Nn]ot[[:space:]]+[Ff]ound ]]
}

@test "audit list filters by --action Write" {
  ws=$(liquid --format json workspace create demo | jq -r '.data.workspace_id')
  export LIQUID_TOKEN=$(liquid --format json auth provision-agent w \
    --workspace "$ws" --role WorkspaceMember | jq -r '.data.token')
  liquid --format json page write /pages/a --workspace "$ws" --data '{"a":1}' >/dev/null
  liquid --format json page write /pages/b --workspace "$ws" --data '{"b":2}' >/dev/null
  run liquid --format json audit list --workspace "$ws" --action Write
  [ "$status" -eq 0 ]
  # NDJSON: each line must have action == "Write"; expect 2 lines.
  count=$(echo "$output" | wc -l)
  [ "$count" -eq 2 ]
  for line in $(echo "$output" | jq -r '.action'); do
    [ "$line" = "Write" ]
  done
}

@test "text format prints human-readable summary for workspace create" {
  run liquid --format text workspace create demo
  [ "$status" -eq 0 ]
  [[ "$output" == *"created workspace"* ]]
  [[ "$output" == *"demo"* ]]
}

@test "text format prints error message on stderr for invalid uuid" {
  liquid --format text workspace create demo >/dev/null
  # Capture stderr separately by routing the command through a subshell.
  run bash -c 'liquid --format text page read /x --workspace not-a-uuid 2>&1 1>/dev/null'
  [ "$status" -eq 2 ]
  [[ "$output" == *"error:"* ]]
  [[ "$output" == *"not a uuid"* ]]
}
