#!/usr/bin/env bats
#
# tests/cli/00_mvp_slice.bats — Phase-1 MVP slice acceptance test.
#
# This file is the single end-to-end spec for the Phase-1 "happy path":
#
#   workspace create
#       → auth provision-agent
#       → page write
#       → page read
#       → audit list
#       → page undo
#       → page read (post-undo)
#
# It currently `skip`s every step with the message "pending M6.5", per
# the M6/M7 split documented in IMPLEMENTATION_PLAN.md §5.7 + §5.8. When
# TASK-008 (Minimal agent CLI) lands, drop each `skip` line as the
# matching subcommand goes green.
#
# Conventions:
#   - Every command must succeed with exit code 0 unless explicitly testing
#     a failure mode.
#   - `--format json` is the agent-friendly output; tests assert on the
#     `.ok` and `.data` fields per §12.
#   - The CLI binary is invoked via `liquid` (PATH) so the same script
#     works whether the binary is `cargo run -p liquid-cli --` or a
#     released artefact.

# shellcheck shell=bash

setup() {
  # Each test gets its own LIQUID_HOME so writes do not bleed across tests.
  export LIQUID_HOME="$(mktemp -d -t liquid-mvp-XXXXXX)"
  export LIQUID_FORMAT="json"
  # If `liquid` is not on PATH (the common local case — only CI
  # installs the binary into /usr/local/bin), fall back to the
  # cargo target dir so a developer can run `bats tests/cli/`
  # right after `cargo build -p liquid-cli`.
  if ! command -v liquid >/dev/null 2>&1; then
    local target_bin="$BATS_TEST_DIRNAME/../../core/target/debug/liquid"
    if [ -x "$target_bin" ]; then
      export PATH="$(dirname "$target_bin"):$PATH"
    fi
  fi
  # Tests that depend on `jq` skip if it is not installed.
  if ! command -v jq >/dev/null 2>&1; then
    skip "jq not installed"
  fi
}

teardown() {
  if [ -n "${LIQUID_HOME:-}" ] && [ -d "$LIQUID_HOME" ]; then
    rm -rf "$LIQUID_HOME"
  fi
}

@test "MVP slice: workspace create returns a workspace id" {
  run liquid --format json workspace create demo-workspace
  [ "$status" -eq 0 ]
  ws_id="$(echo "$output" | jq -r '.data.workspace_id')"
  [ -n "$ws_id" ] && [ "$ws_id" != "null" ]
}

@test "MVP slice: auth provision-agent returns a usable token" {
  ws_id="$(liquid --format json workspace create demo-workspace | jq -r '.data.workspace_id')"
  run liquid --format json auth provision-agent demo-agent \
    --workspace "$ws_id" --role WorkspaceMember
  [ "$status" -eq 0 ]
  token="$(echo "$output" | jq -r '.data.token')"
  [ -n "$token" ] && [ "$token" != "null" ]
  # Token format per ADR-002: principal . expires_unix . hmac_hex
  [[ "$token" =~ ^a:[0-9a-f-]+\.[0-9]+\.[0-9a-f]+$ ]]
}

@test "MVP slice: page write then read round-trip" {

  ws_id="$(liquid --format json workspace create demo-workspace | jq -r '.data.workspace_id')"
  # Page writes need a role whose permission matrix permits Write on
  # `Resource::Page` — `WorkspaceMember` does, per §9 / role.rs. The
  # `AppViewer` / `AppEditor` family is AppInstance-scoped and is
  # exercised in the negative test at the bottom of this file.
  export LIQUID_TOKEN="$(liquid --format json auth provision-agent demo-agent \
    --workspace "$ws_id" --role WorkspaceMember | jq -r '.data.token')"

  payload='{"title":"hello","body":"world"}'

  run liquid --format json page write /pages/welcome \
    --workspace "$ws_id" --data "$payload"
  [ "$status" -eq 0 ]
  op_id="$(echo "$output" | jq -r '.data.operation_id')"
  [ -n "$op_id" ] && [ "$op_id" != "null" ]

  run liquid --format json page read /pages/welcome --workspace "$ws_id"
  [ "$status" -eq 0 ]
  [ "$(echo "$output" | jq -r '.data.title')" = "hello" ]
  [ "$(echo "$output" | jq -r '.data.body')"  = "world" ]
}

@test "MVP slice: audit list surfaces the prior write" {

  ws_id="$(liquid --format json workspace create demo-workspace | jq -r '.data.workspace_id')"
  # Page writes need a role whose permission matrix permits Write on
  # `Resource::Page` — `WorkspaceMember` does, per §9 / role.rs. The
  # `AppViewer` / `AppEditor` family is AppInstance-scoped and is
  # exercised in the negative test at the bottom of this file.
  export LIQUID_TOKEN="$(liquid --format json auth provision-agent demo-agent \
    --workspace "$ws_id" --role WorkspaceMember | jq -r '.data.token')"

  liquid --format json page write /pages/welcome \
    --workspace "$ws_id" --data '{"v":1}' >/dev/null

  run liquid --format json audit list --workspace "$ws_id"
  [ "$status" -eq 0 ]
  # The op_log emits one record per mutation; the most recent should be
  # the Write we just performed, attributed to the demo-agent principal.
  latest="$(echo "$output" | tail -n 1 | jq .)"
  [ "$(echo "$latest" | jq -r '.action')" = "Write" ]
  [ "$(echo "$latest" | jq -r '.path')"   = "/pages/welcome" ]
  [[ "$(echo "$latest" | jq -r '.principal')" =~ ^a:[0-9a-f-]+$ ]]
}

@test "MVP slice: page undo reverses the most recent write" {

  ws_id="$(liquid --format json workspace create demo-workspace | jq -r '.data.workspace_id')"
  # Page writes need a role whose permission matrix permits Write on
  # `Resource::Page` — `WorkspaceMember` does, per §9 / role.rs. The
  # `AppViewer` / `AppEditor` family is AppInstance-scoped and is
  # exercised in the negative test at the bottom of this file.
  export LIQUID_TOKEN="$(liquid --format json auth provision-agent demo-agent \
    --workspace "$ws_id" --role WorkspaceMember | jq -r '.data.token')"

  op_id="$(liquid --format json page write /pages/welcome \
    --workspace "$ws_id" --data '{"v":1}' | jq -r '.data.operation_id')"

  run liquid --format json page undo /pages/welcome \
    --workspace "$ws_id" --op "$op_id"
  [ "$status" -eq 0 ]

  # Post-undo, the page must not exist — read should return a non-zero
  # exit code and an `error` payload (NotFound mapped to a friendly
  # message; never leak internal store paths).
  run liquid --format json page read /pages/welcome --workspace "$ws_id"
  [ "$status" -ne 0 ]
  [ "$(echo "$output" | jq -r '.ok')" = "false" ]
  [[ "$(echo "$output" | jq -r '.error')" =~ [Nn]ot[[:space:]]+[Ff]ound ]]
}

@test "MVP slice: AppViewer is rejected on page write (Absolute Rule 4)" {
  ws_id="$(liquid --format json workspace create demo-workspace | jq -r '.data.workspace_id')"
  # AppViewer is AppInstance-scoped per §9 / role.rs. The CLI does
  # not generate an AppInstance for us (Phase-1 has no `app
  # install` subcommand yet — M7), so we supply a fresh UUID as
  # the scope. The role still doesn't permit Write on Page, which
  # is what the negative test exists to prove.
  scope_uuid="$(uuidgen 2>/dev/null || python3 -c 'import uuid; print(uuid.uuid4())')"
  viewer_token="$(liquid --format json auth provision-agent demo-viewer \
    --workspace "$ws_id" --role AppViewer --scope "$scope_uuid" | jq -r '.data.token')"
  [ -n "$viewer_token" ] && [ "$viewer_token" != "null" ]

  LIQUID_TOKEN="$viewer_token" run liquid --format json page write /pages/welcome \
    --workspace "$ws_id" --data '{"v":1}'
  [ "$status" -ne 0 ]
  [ "$(echo "$output" | jq -r '.ok')" = "false" ]
  [ "$(echo "$output" | jq -r '.error')" = "Forbidden" ]
}
