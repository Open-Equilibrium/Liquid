#!/usr/bin/env bats
#
# tests/cli/11_m7_full_cli.bats — focused per-subcommand tests for
# the M7 `liquid` CLI surface (TASK-009). Builds on M6.5 with the
# remainder of §12 (excluding `app …` which depends on M8's
# AppManifest):
#
#   - liquid workspace list
#   - liquid workspace delete <id>
#   - liquid page history <page-path>
#   - liquid auth login --username <u> --password <p>
#   - liquid auth whoami
#   - global --as <name|principal-id> impersonation flag

# shellcheck shell=bash

setup() {
  export LIQUID_HOME="$(mktemp -d -t liquid-m7-XXXXXX)"
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

# ── workspace list ─────────────────────────────────────────────────────────

@test "workspace list returns an empty NDJSON stream on a fresh state root" {
  run liquid --format json workspace list
  [ "$status" -eq 0 ]
  # `workspace list` bootstraps + then lists the principal's workspaces.
  # Fresh state ⇒ no workspaces (the bootstrap user has no bindings yet).
  count=$(echo "$output" | grep -c . || true)
  [ "$count" -eq 0 ]
}

@test "workspace list returns the workspaces the caller can read" {
  liquid --format json workspace create alpha >/dev/null
  liquid --format json workspace create beta >/dev/null
  run liquid --format json workspace list
  [ "$status" -eq 0 ]
  count=$(echo "$output" | wc -l)
  [ "$count" -eq 2 ]
  for name in $(echo "$output" | jq -r '.name'); do
    [[ "$name" =~ ^(alpha|beta)$ ]]
  done
}

# ── workspace delete ─────────────────────────────────────────────────────

@test "workspace delete removes the workspace from the registry and stays auth-gated" {
  ws=$(liquid --format json workspace create demo | jq -r .data.workspace_id)
  run liquid --format json workspace delete "$ws"
  [ "$status" -eq 0 ]
  [ "$(echo "$output" | jq -r '.ok')" = "true" ]
  # Post-delete: list returns 0 records.
  run liquid --format json workspace list
  [ "$status" -eq 0 ]
  count=$(echo "$output" | grep -c . || true)
  [ "$count" -eq 0 ]
}

@test "workspace delete rejects an unknown workspace with Forbidden" {
  # The permission check fires before the registry lookup: the
  # caller has no Admin binding on a workspace they never created.
  # Per §4.5 we surface Forbidden rather than NotFound — leaking
  # "no such workspace" lets an attacker enumerate ids.
  liquid --format json workspace create demo >/dev/null
  bogus="$(uuidgen 2>/dev/null || python3 -c 'import uuid; print(uuid.uuid4())')"
  run liquid --format json workspace delete "$bogus"
  [ "$status" -eq 1 ]
  [ "$(echo "$output" | jq -r '.ok')" = "false" ]
  [ "$(echo "$output" | jq -r '.error')" = "Forbidden" ]
}

@test "workspace delete rejects a non-owner with Forbidden" {
  ws=$(liquid --format json workspace create demo | jq -r .data.workspace_id)
  outsider=$(liquid --format json auth provision-agent intruder \
    --workspace "$ws" --role WorkspaceMember | jq -r .data.token)
  LIQUID_TOKEN="$outsider" run liquid --format json workspace delete "$ws"
  [ "$status" -eq 1 ]
  [ "$(echo "$output" | jq -r '.error')" = "Forbidden" ]
}

# ── page history ─────────────────────────────────────────────────────────

@test "page history returns one record per write to the path, newest first" {
  ws=$(liquid --format json workspace create demo | jq -r .data.workspace_id)
  export LIQUID_TOKEN=$(liquid --format json auth provision-agent w \
    --workspace "$ws" --role WorkspaceMember | jq -r .data.token)
  liquid --format json page write /pages/a --workspace "$ws" --data '{"v":1}' >/dev/null
  liquid --format json page write /pages/a --workspace "$ws" --data '{"v":2}' >/dev/null
  liquid --format json page write /pages/b --workspace "$ws" --data '{"v":3}' >/dev/null
  run liquid --format json page history /pages/a --workspace "$ws"
  [ "$status" -eq 0 ]
  # Two writes to /pages/a, none for /pages/b → 2 records here.
  count=$(echo "$output" | wc -l)
  [ "$count" -eq 2 ]
  # NDJSON newest-first; first line ⇒ newest write.
  first=$(echo "$output" | head -n 1)
  [ "$(echo "$first" | jq -r '.path')" = "/pages/a" ]
  [ "$(echo "$first" | jq -r '.action')" = "Write" ]
}

@test "page history respects --limit" {
  ws=$(liquid --format json workspace create demo | jq -r .data.workspace_id)
  export LIQUID_TOKEN=$(liquid --format json auth provision-agent w \
    --workspace "$ws" --role WorkspaceMember | jq -r .data.token)
  for i in 1 2 3 4 5; do
    liquid --format json page write /pages/p --workspace "$ws" --data "{\"v\":$i}" >/dev/null
  done
  run liquid --format json page history /pages/p --workspace "$ws" --limit 3
  [ "$status" -eq 0 ]
  count=$(echo "$output" | wc -l)
  [ "$count" -eq 3 ]
}

@test "page history with --limit > matches returns all path-matching writes only" {
  # Regression for the audit's MT4: when other paths dominate the op
  # log, the --limit cap on the target path must apply per-path, not
  # as a prefix cap on the log. We mix 5 writes to /pages/target with
  # 5 writes to /pages/noise and ask for --limit 20: we must get 5.
  ws=$(liquid --format json workspace create demo | jq -r .data.workspace_id)
  export LIQUID_TOKEN=$(liquid --format json auth provision-agent w \
    --workspace "$ws" --role WorkspaceMember | jq -r .data.token)
  for i in 1 2 3 4 5; do
    liquid --format json page write /pages/target --workspace "$ws" --data "{\"v\":$i}" >/dev/null
    liquid --format json page write /pages/noise --workspace "$ws" --data "{\"v\":$i}" >/dev/null
  done
  run liquid --format json page history /pages/target --workspace "$ws" --limit 20
  [ "$status" -eq 0 ]
  count=$(echo "$output" | wc -l)
  [ "$count" -eq 5 ]
  # Every returned record points at /pages/target — no /pages/noise
  # entries leaked through the filter.
  for path in $(echo "$output" | jq -r '.path'); do
    [ "$path" = "/pages/target" ]
  done
}

# ── auth login ───────────────────────────────────────────────────────────

@test "auth login non-interactive flags issue a token for an existing user" {
  # First, bootstrap so the default 'cli' user exists.
  liquid --format json workspace create demo >/dev/null
  # Now login as a fresh principal.
  liquid --format json auth login --username alice --password 'pw-alice' --register >/dev/null
  rm -f "$LIQUID_HOME/token"
  run liquid --format json auth login --username alice --password 'pw-alice'
  [ "$status" -eq 0 ]
  token=$(echo "$output" | jq -r '.data.token')
  [[ "$token" =~ ^u:[0-9a-f-]+\.[0-9]+\.[0-9a-f]+$ ]]
  # Token file written, equals the printed token.
  [ "$(cat $LIQUID_HOME/token)" = "$token" ]
}

@test "auth login --register rejects a duplicate username with InvalidInput" {
  # Regression for the audit's MT2: a second --register with an
  # already-claimed username must not silently overwrite the first
  # user's credentials. InvalidInput maps to exit 2 (EX_USAGE) per
  # `output::exit_code_for`.
  liquid --format json workspace create demo >/dev/null
  liquid --format json auth login --username carol --password 'first-pw' --register >/dev/null
  run liquid --format json auth login --username carol --password 'second-pw' --register
  [ "$status" -eq 2 ]
  err=$(echo "$output" | jq -r '.error')
  [[ "$err" =~ already[[:space:]]+registered ]]
}

@test "auth login rejects wrong password with Forbidden" {
  liquid --format json workspace create demo >/dev/null
  liquid --format json auth login --username bob --password 'right-pw' --register >/dev/null
  run liquid --format json auth login --username bob --password 'wrong-pw'
  [ "$status" -eq 1 ]
  [ "$(echo "$output" | jq -r '.error')" = "Forbidden" ]
}

# ── auth whoami ──────────────────────────────────────────────────────────

@test "auth whoami prints the resolved principal" {
  liquid --format json workspace create demo >/dev/null
  run liquid --format json auth whoami
  [ "$status" -eq 0 ]
  [ "$(echo "$output" | jq -r '.ok')" = "true" ]
  principal=$(echo "$output" | jq -r '.data.principal')
  # Bootstrap user, so principal kind is `user`.
  [[ "$principal" =~ ^user:[0-9a-f-]+$ ]]
  kind=$(echo "$output" | jq -r '.data.kind')
  [ "$kind" = "user" ]
}

@test "auth whoami without a token errors with InvalidInput" {
  unset LIQUID_TOKEN
  run liquid --format json auth whoami
  [ "$status" -eq 2 ]
  [ "$(echo "$output" | jq -r '.ok')" = "false" ]
}

# ── --as impersonation ───────────────────────────────────────────────────

@test "--as <agent-name> impersonates the named agent" {
  ws=$(liquid --format json workspace create demo | jq -r .data.workspace_id)
  member=$(liquid --format json auth provision-agent worker \
    --workspace "$ws" --role WorkspaceMember | jq -r .data.agent_id)
  # The owner token (LIQUID_HOME/token) is the workspace owner.
  # Use --as to act as `worker`. Owner→worker delegation must be
  # auth-gated: only the workspace owner (or an admin) can use --as
  # on agents inside the workspace.
  run liquid --format json --as "worker" page write /pages/x \
    --workspace "$ws" --data '{"hello":"world"}'
  [ "$status" -eq 0 ]
  # The audit log records the agent as the author, not the owner.
  run liquid --format json audit list --workspace "$ws"
  [ "$status" -eq 0 ]
  latest=$(echo "$output" | head -n 1)
  [ "$(echo "$latest" | jq -r '.principal')" = "a:$member" ]
}

@test "--as rejects an unknown name with NotFound" {
  ws=$(liquid --format json workspace create demo | jq -r .data.workspace_id)
  run liquid --format json --as "nonexistent-bot" workspace list
  [ "$status" -eq 1 ]
  [[ "$(echo "$output" | jq -r '.error')" =~ [Nn]ot[[:space:]]+[Ff]ound ]]
}

@test "--as rejects an ambiguous name with InvalidInput pointing at the principal-form" {
  # Regression for the audit's MT3: when two agents share a name
  # across workspaces, --as must refuse with InvalidInput (exit 2,
  # EX_USAGE) and tell the caller to use the principal-form
  # (a:<uuid>) to disambiguate. The error message is in the
  # envelope's `error` field, not a separate `message`.
  ws1=$(liquid --format json workspace create alpha | jq -r .data.workspace_id)
  ws2=$(liquid --format json workspace create beta | jq -r .data.workspace_id)
  liquid --format json auth provision-agent dup \
    --workspace "$ws1" --role WorkspaceMember >/dev/null
  liquid --format json auth provision-agent dup \
    --workspace "$ws2" --role WorkspaceMember >/dev/null
  run liquid --format json --as "dup" workspace list
  [ "$status" -eq 2 ]
  err=$(echo "$output" | jq -r '.error')
  [[ "$err" =~ multiple[[:space:]]+agents ]]
  # The error message guides the caller to disambiguate with a:<uuid>.
  [[ "$err" =~ a:\<uuid\> ]] || [[ "$err" =~ a: ]]
}
