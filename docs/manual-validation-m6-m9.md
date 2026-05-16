# Manual Validation — Phase-2 Milestones M6, M7, M8, M9

Auditable companion to the automated test suite for the
Phase-1→Phase-2 transition: M6 (Flutter shell), M7 (full agent
CLI), M8 (public Dart SDK), and M9 (data binding broker — Rust
side). M10 (multi-instance tenant configuration) ships under
TASK-017 and gets its own guide once it lands.

Read this after
[`manual-validation-m4-m5.md`](manual-validation-m4-m5.md) and
[`manual-validation-m6.5.md`](manual-validation-m6.5.md). M6
hosts the M6.5 CLI's data path in a desktop UI; M7 extends the
M6.5 CLI surface; M8/M9 build the SDK + slot-bus foundation
every Phase-2 first-party app depends on.

## Why a manual guide if the automated suite passes?

`cargo test` + `flutter test` + `bats` prove the asserted
behaviours pass. The manual walkthrough catches a different
class of regression:

- **Surface drift** — does the M8 SDK still expose the typed
  shapes app developers extend? Renaming a public class without
  a `--breaking-change` audit is the kind of regression only an
  eyes-on read catches.
- **Cross-stack contract drift** — does `SlotValue` on the Rust
  side still mirror `SlotValue` on the Dart side? Both layers
  serialise into the same wire payload; mismatched variants
  would silently break apps.
- **UI affordance regressions** — does the M6 shell still mount
  the four canonical widgets? A misnamed widget can pass
  `flutter test` if the test was renamed to match.

Run this whenever you cut a release tag, merge an M6-M9 PR, or
hand the project off to a new maintainer.

---

## Prerequisites

| Tool | Version | Why |
|---|---|---|
| Rust | `1.94.1` (pinned) | M7 + M9 Rust side. |
| `bats` + `jq` | latest | M7 CLI tests. |
| Flutter | stable channel ≥ 3.24 | M6 + M8. |

```sh
cd <repo-root>
cargo build --manifest-path core/Cargo.toml --workspace
flutter pub get --directory app
flutter pub get --directory sdk/liquid_sdk
```

---

## M6 — Flutter shell skeleton (`app/`)

**Spec:** `IMPLEMENTATION_PLAN.md §5.7`. **Success criterion:**
the 4 widget tests in `app/test/widget_test.dart` pass (visual
+ drag-on-real-display validation is deferred per CLAUDE.md's
cloud-session limitation).

### Step M6.1 — Focused tests

```sh
cd app && flutter test 2>&1 | tail -10
```

**Expected:** 4 / 4 cases pass:
1. `RootShell mounts and renders the four canonical widgets`
2. `Workspace switcher lists two demo workspaces and PageArea
   shows the selected name`
3. `PageGrid hosts the placeholder GridItem on first launch`
4. `Toolbar shows add-item button (active) + save/history
   (pending)`

### Step M6.2 — Surface invariants by inspection

```sh
grep -nE '^(class|abstract class) ' app/lib/src/*.dart
```

Confirm by eye every M6 widget exists:

- `RootShell` (`root_shell.dart`)
- `ExplorerPanel` (`explorer_panel.dart`)
- `PageArea` (`page_area.dart`)
- `PageGrid` + `GridItem` (`page_grid.dart`)
- `WorkspaceSummary` + `workspacesProvider` +
  `currentWorkspaceProvider` + `gridItemsProvider` (`state.dart`)

### Step M6.3 — Live launch (optional, real display only)

```sh
cd app && flutter run -d linux
```

Drag the left-panel divider; switch workspaces; drag the
placeholder grid item; resize via the bottom-right handle.
Expected: every interaction snaps to the 12×12 grid. This step
is skipped in the cloud session per CLAUDE.md.

---

## M7 — Full agent CLI (`liquid-cli` extensions)

**Spec:** `IMPLEMENTATION_PLAN.md §5.8`. **Success criterion:**
`bats tests/cli/11_m7_full_cli.bats` reports 16 / 16. The
`app …` subset is carved out to TASK-014 (depends on M8's
`AppManifest`).

### Step M7.1 — Focused tests

```sh
bats tests/cli/11_m7_full_cli.bats 2>&1 | tail -20
```

**Expected:** 16 / 16 cases. Coverage breakdown:
- workspace list — empty + 2-workspace happy path (2 cases)
- workspace delete — happy + non-owner Forbidden + unknown id
  Forbidden (3 cases)
- page history — happy + `--limit` cap + `--limit > matches`
  per-path cap (3 cases — the third is a PR #18 audit-pass
  regression for path-filter semantics)
- auth login — happy + wrong-password Forbidden + duplicate
  `--register` InvalidInput (3 cases — the third is a PR #18
  audit-pass regression for username uniqueness)
- auth whoami — happy + no-token InvalidInput (2 cases)
- `--as` — happy + unknown-name NotFound + ambiguous-name
  InvalidInput (3 cases — the third is a PR #18 audit-pass
  regression for cross-workspace name collisions)

### Step M7.2 — Manual sanity walkthrough

```sh
export LIQUID_HOME="$(mktemp -d)"
export LIQUID_FORMAT=json
WS=$(./core/target/debug/liquid workspace create demo | jq -r .data.workspace_id)
./core/target/debug/liquid workspace list | jq .
./core/target/debug/liquid auth whoami | jq .
./core/target/debug/liquid workspace delete "$WS"
./core/target/debug/liquid workspace list | jq .   # empty
```

**Expected:** the second `workspace list` returns an empty
NDJSON stream (the workspace was just removed from the registry).

### Step M7.3 — Anti-enumeration check

```sh
WS=$(./core/target/debug/liquid workspace create demo | jq -r .data.workspace_id)
BOGUS=$(uuidgen 2>/dev/null || python3 -c 'import uuid; print(uuid.uuid4())')
./core/target/debug/liquid workspace delete "$BOGUS"; echo "exit=$?"
```

**Expected:** exit `1`, `.error == "Forbidden"` — NOT
`"Not found"`. The permission check fires before the registry
lookup so an attacker cannot enumerate workspace IDs. See §4.5.

---

## M8 — Public Dart SDK (`sdk/liquid_sdk/`)

**Spec:** `IMPLEMENTATION_PLAN.md §6.1`. **Success criterion:**
the M8 acceptance test
(`sdk/liquid_sdk/test/liquid_sdk_test.dart`) defines a stub
`LiquidComponent` declaring two slots and compiles + passes.

### Step M8.1 — Focused tests + analyzer

```sh
cd sdk/liquid_sdk && flutter test 2>&1 | tail -5
flutter analyze 2>&1 | tail -3
```

**Expected:** 8 / 8 test cases pass (the original 6 plus the
PR #18 audit-pass `SlotValue.json` and `SlotValue.bytes`
structural-equality regressions). `flutter analyze` reports
`No issues found!`.

### Step M8.2 — API surface inventory

```sh
grep -nE '^(class|abstract class|sealed class|enum) ' sdk/liquid_sdk/lib/src/*.dart
```

**Expected:** every name from §6.1's checklist appears:

- `slot.dart` — sealed `SlotValue` + `SlotKind` + `SlotSchema`
  + `InputSlot` + `OutputSlot`
- `component.dart` — `GridConstraints` + abstract
  `LiquidComponent`
- `manifest.dart` — `ManifestAction` enum + `Permission` +
  `TenantConfigSchema` + `CliCommandDeclaration` +
  `ComponentManifest` + `AppManifest`
- `runtime_apis.dart` — `GridApi` + `VcsApi` + `HistoryEntry` +
  `PermissionApi` + `SlotEmitter` + `SlotConsumer`

### Step M8.3 — Cross-stack `SlotValue` parity

```sh
grep -nE 'SlotValue::Str|SlotValue::Num|SlotValue::Bool|SlotValue::Json|SlotValue::Bytes' \
  core/liquid-core/src/slot.rs
grep -nE 'SlotValue\.(str|num|bool|json|bytes)' sdk/liquid_sdk/lib/src/slot.dart
```

**Expected:** the same 5 variants on each side. Any new variant
added to one stack but not the other = silent serde drift; fix
before merge.

---

## M9 — Data binding broker (`liquid-bindings` Rust side)

**Spec:** `IMPLEMENTATION_PLAN.md §6.2`. **Success criterion
(Rust side):** the 12 inline tests in
`core/liquid-bindings/src/broker.rs` cover the §6.2 contract.
The "spreadsheet emits → chart updates" demonstration is the
Dart-side success criterion, deferred to TASK-012 + TASK-016b.

### Step M9.1 — Focused tests

```sh
cargo test --manifest-path core/Cargo.toml -p liquid-bindings \
  2>&1 | .claude/hooks/filter-test-output.sh
```

**Expected:** 12 / 12 cases pass:
- `publish_with_no_subscribers_returns_zero`
- `subscribe_then_publish_delivers_one_message`
- `two_subscribers_each_get_their_own_copy`
- `wire_routes_publishes_to_downstream_subscribers`
- `wire_rejects_self_loop`
- `wire_is_idempotent`
- `wire_rejects_multi_hop_cycle` (PR #18 audit regression — 2-hop)
- `wire_rejects_three_hop_cycle` (PR #18 audit regression — 3-hop)
- `save_then_load_round_trips_the_wiring_document`
- `load_bindings_rejects_self_wires`
- `load_bindings_rejects_multi_hop_cycle` (PR #18 audit regression)
- `bindings_document_round_trips_json`

### Step M9.2 — Surface invariants by inspection

```sh
grep -nE '^(pub )?(trait|struct|enum|fn|const) ' \
  core/liquid-bindings/src/broker.rs | head -20
```

**Expected:** the `SlotBroker` trait + 5 methods (`publish`,
`subscribe`, `wire`, `load_bindings`, `save_bindings`); the
`InProcessSlotBroker` struct + `new()` + `lock()` +
`sender_for()` helpers; `SlotWiring` + `BindingsDocument` +
`SharedBroker` (= `Arc<dyn SlotBroker>`); `SLOT_BUFFER_SIZE`
constant.

### Step M9.3 — Persistence-replay contract

The §6.2 spec says: "wiring is replayed on page load — all slot
subscriptions are re-established". The Rust side proves the
round-trip via the `save_then_load_round_trips_the_wiring_document`
test. To re-verify manually:

```sh
cargo test --manifest-path core/Cargo.toml \
  -p liquid-bindings \
  -- --exact \
  broker::tests::save_then_load_round_trips_the_wiring_document
```

**Expected:** the test exercises a fresh broker, replays the
saved document, publishes on the original `from` slot, and
confirms a subscriber on the wired `to` receives the value.
If this fails, page-reload wiring is broken — block the merge.

---

## Sign-off checklist

Tick every box before stamping the run-log:

- [ ] M6 — `flutter test` 4 / 4; `flutter analyze` clean.
- [ ] M7 — `bats tests/cli/11_m7_full_cli.bats` 16 / 16
      (13 shipped with M7 + 3 PR #18 audit regressions);
      anti-enumeration sanity (§M7.3) returns `Forbidden`
      not `Not found`.
- [ ] M8 — `flutter test` 8 / 8 in `sdk/liquid_sdk/`
      (6 shipped + 2 `SlotValue.json` / `SlotValue.bytes`
      structural-equality regressions); `flutter analyze` clean;
      `SlotValue` variant parity between Rust + Dart.
- [ ] M9 Rust side — 12 / 12 inline tests (9 shipped + 2-hop
      + 3-hop `wire` cycle rejection + multi-hop `load_bindings`
      cycle rejection); `save_then_load_round_trips_the_wiring_document`
      green.
- [ ] M9 Dart side — STATUS still PENDING (TASK-012 + TASK-016b).
- [ ] Cross-milestone — `cargo test --workspace --locked`
      green; `cargo clippy --workspace --all-targets --locked --
      -D warnings` clean; `cargo fmt --all --check` clean;
      `cargo deny check` clean; `just coverage-check` clean.
- [ ] `bats tests/cli/` full suite passes (120 / 120 after the
      PR #18 audit-pass regressions).

If any line above is unchecked, the milestone is **not** done.

---

## Related documents

- [`manual-validation-m1-m3.md`](manual-validation-m1-m3.md)
- [`manual-validation-m4-m5.md`](manual-validation-m4-m5.md)
- [`manual-validation-m6.5.md`](manual-validation-m6.5.md)
- `IMPLEMENTATION_PLAN.md §5.7 / §5.8 / §6.1 / §6.2` — the
  authoritative spec.
- `app/test/widget_test.dart` — M6 success-criterion tests.
- `tests/cli/11_m7_full_cli.bats` — M7 success-criterion suite.
- `sdk/liquid_sdk/test/liquid_sdk_test.dart` — M8
  success-criterion suite.
- `core/liquid-bindings/src/broker.rs` — M9 Rust side + 9
  inline tests.
- `CHANGELOG.md` — every M6-M9 surface change ships with a
  matching `## [Unreleased]` entry.
