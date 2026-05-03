# Liquid — Implementation Plan

This document is the authoritative guide for implementing the Liquid UI framework.
It is written for AI agents and human developers picking up the project cold.
Read the [README](./README.md) first for the full concept; this document is purely
about *how to build it*.

---

## Table of Contents

1. [Guiding Principles for Implementors](#1-guiding-principles-for-implementors)
2. [Repository Layout](#2-repository-layout)
3. [Architecture Overview](#3-architecture-overview)
4. [Core Interfaces — Define These First](#4-core-interfaces--define-these-first)
5. [Phase 1 — Rust Core + Flutter Shell Skeleton](#5-phase-1--rust-core--flutter-shell-skeleton)
6. [Phase 2 — SDK + First-Party Apps](#6-phase-2--sdk--first-party-apps)
7. [Phase 3 — Mobile + Scale + Extensions](#7-phase-3--mobile--scale--extensions)
8. [Phase 4 — Ecosystem + High Availability](#8-phase-4--ecosystem--high-availability)
9. [Crate Reference](#9-crate-reference)
10. [Flutter Application Reference](#10-flutter-application-reference)
11. [SDK Design Specification](#11-sdk-design-specification)
12. [Agent CLI Specification](#12-agent-cli-specification)
13. [Data Binding Protocol](#13-data-binding-protocol)
14. [Testing Strategy](#14-testing-strategy)
15. [Key Design Decisions](#15-key-design-decisions)

---

## 1. Guiding Principles for Implementors

These rules take precedence over any implementation convenience.

**Interfaces before implementations.**
Every layer boundary (Rust crate ↔ crate, Rust ↔ Dart, SDK ↔ app) must have its
interface defined and reviewed before any implementation begins. An interface
that is wrong costs a rewrite; an interface that is right costs only time.

**Workspace partitioning is non-negotiable from the first line of storage code.**
Every storage and permission call takes a `WorkspaceId`. There is no global
namespace. Adding workspace isolation later requires rewriting every storage
callsite — do not defer it.

**The cache and permission index are behind interfaces from day one.**
Phase 1 uses in-process stub implementations. Phase 3 swaps them for distributed
ones. This swap must require zero changes to application code. If it does require
changes, the interface boundary was drawn in the wrong place.

**The Dart layer renders and routes input only.**
All business logic — VCS reads/writes, permission checks, agent auth, data slot
routing — lives in Rust and is called via `flutter_rust_bridge`. Dart code that
directly manages state beyond UI state is a boundary violation.

**Security is not a phase.**
Signed manifests, capability checks, and zero-trust between components are
implemented in phase 1. They are not added later. A public registry opening on
an unsigned, unaudited codebase is a critical failure mode.

---

## 2. Repository Layout

```
liquid/
├── core/                          # Rust Cargo workspace
│   ├── Cargo.toml                 # workspace manifest
│   ├── liquid-core/               # shared primitives, error types, IDs
│   ├── liquid-vcs/                # Jujutsu wrapper + content store abstraction
│   ├── liquid-auth/               # identity, OIDC, session management
│   ├── liquid-permissions/        # RBAC model + materialized index
│   ├── liquid-cache/              # content-addressable cache abstraction
│   ├── liquid-bindings/           # data binding pub/sub broker
│   ├── liquid-sdk-bridge/         # flutter_rust_bridge FFI exports
│   └── liquid-cli/                # `liquid` agent CLI binary
│
├── app/                           # Flutter application
│   ├── pubspec.yaml
│   ├── lib/
│   │   ├── main.dart
│   │   ├── bridge/                # generated flutter_rust_bridge bindings (do not edit)
│   │   ├── shell/                 # WorkspaceSwitcher, RootShell layout
│   │   ├── explorer/              # ExplorerPanel, PageTree, AppInstanceList
│   │   ├── grid/                  # PageGrid, GridCell, GridItem
│   │   ├── pages/                 # Page model, PageView
│   │   ├── bindings/              # Dart-side data binding wiring UI
│   │   └── state/                 # Riverpod providers (UI state only)
│   └── test/
│
├── sdk/                           # Public Dart package for app developers
│   └── liquid_sdk/
│       ├── pubspec.yaml
│       └── lib/
│           ├── manifest.dart      # AppManifest, ComponentManifest
│           ├── component.dart     # LiquidComponent base class
│           ├── slots.dart         # InputSlot, OutputSlot, SlotSchema
│           ├── grid.dart          # GridConstraints, GridApi
│           ├── vcs.dart           # VcsApi
│           ├── permissions.dart   # PermissionApi
│           └── extensions.dart    # ExtensionPoint, ExtensionApi
│
├── registry/                      # Self-hosted package registry (Rust)
│   └── liquid-registry/
│
├── docs/
│   ├── adr/                       # Architecture Decision Records
│   └── sdk-guide/                 # Developer guide for app builders
│
├── apps/                          # First-party reference apps (Dart)
│   ├── text_editor/
│   ├── spreadsheet/
│   └── chart/
│
├── README.md
└── IMPLEMENTATION_PLAN.md         # this file
```

---

## 3. Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│  Flutter App  (Dart — rendering and input only)             │
│  ┌──────────┐  ┌──────────────┐  ┌─────────────────────┐  │
│  │  Shell   │  │   Explorer   │  │   Page / Grid       │  │
│  │ (switch  │  │ (page tree,  │  │ (GridItem widgets,  │  │
│  │ workspace│  │  app list)   │  │  slot wiring UI)    │  │
│  └──────────┘  └──────────────┘  └─────────────────────┘  │
│                     flutter_rust_bridge FFI                  │
└───────────────────────────┬─────────────────────────────────┘
                            │  async Rust calls via Future/Stream
┌───────────────────────────▼─────────────────────────────────┐
│  liquid-sdk-bridge  (FFI surface — thin, no logic)          │
└───┬───────────────┬───────────────┬───────────────┬─────────┘
    │               │               │               │
┌───▼───┐     ┌─────▼────┐   ┌─────▼────┐   ┌─────▼────────┐
│liquid │     │ liquid-  │   │ liquid-  │   │ liquid-      │
│-vcs   │     │ auth     │   │ permiss- │   │ bindings     │
│       │     │          │   │ ions     │   │ (pub/sub     │
│Jujutsu│     │OIDC/RBAC │   │ index    │   │  broker)     │
│wrapper│     │ sessions │   │          │   │              │
└───┬───┘     └──────────┘   └──────────┘   └──────────────┘
    │
┌───▼─────────────────────────────────────────────────────────┐
│  liquid-cache   (content-addressable read cache)             │
│  Phase 1: in-process HashMap   Phase 3: Redis-class          │
└───┬─────────────────────────────────────────────────────────┘
    │
┌───▼─────────────────────────────────────────────────────────┐
│  liquid-core  (WorkspaceId, AppInstanceId, TenantConfig,     │
│               ComponentId, PrincipalId, ContentHash, …)      │
└─────────────────────────────────────────────────────────────┘
```

**Data flow for a page load:**

1. Dart calls `bridge.loadPage(workspaceId, pageId)` via FFI
2. `liquid-sdk-bridge` calls `liquid-permissions` — checks read access (O(1) index lookup)
3. If permitted, calls `liquid-cache` — returns page bytes if warm
4. On cache miss, calls `liquid-vcs` — reads from Jujutsu, warms cache
5. Returns `PageSnapshot` (serialised) to Dart
6. Dart deserialises and renders `GridItem` widgets

**Data flow for a component data binding event:**

1. Component A (Dart widget) emits a value on its output slot via SDK
2. SDK calls `bridge.publishSlot(instanceId, slotName, value)`
3. `liquid-bindings` broker fans out to all registered subscribers for that slot
4. Component B (Dart widget) receives value via `Stream` and re-renders

---

## 4. Core Interfaces — Define These First

These traits must be finalised before any implementation that depends on them.
An agent working on a downstream crate must not assume an interface — open a
discussion or check `docs/adr/` for the rationale.

### 4.1 ContentStore (`liquid-vcs`)

```rust
#[async_trait]
pub trait ContentStore: Send + Sync {
    /// Read the current content of `path` in `workspace`.
    async fn read(
        &self,
        workspace: WorkspaceId,
        path: &StorePath,
    ) -> Result<Bytes, StoreError>;

    /// Atomically write `content` to `path`, creating a commit attributed to
    /// `author` with `message`. Returns the new CommitId.
    async fn write(
        &self,
        workspace: WorkspaceId,
        path: &StorePath,
        content: Bytes,
        author: PrincipalId,
        message: &str,
    ) -> Result<CommitId, StoreError>;

    /// Return the ordered operation log for `workspace` (newest first).
    async fn operation_log(
        &self,
        workspace: WorkspaceId,
        limit: usize,
    ) -> Result<Vec<Operation>, StoreError>;

    /// Undo the operation identified by `op_id`. Returns the resulting CommitId.
    async fn undo(
        &self,
        workspace: WorkspaceId,
        op_id: OperationId,
    ) -> Result<CommitId, StoreError>;

    /// List children of `path` (directory-style listing).
    async fn list(
        &self,
        workspace: WorkspaceId,
        path: &StorePath,
    ) -> Result<Vec<StorePath>, StoreError>;
}
```

### 4.2 PermissionIndex (`liquid-permissions`)

```rust
#[async_trait]
pub trait PermissionIndex: Send + Sync {
    /// Returns true if `principal` may perform `action` on `resource`.
    /// Must complete in < 1 ms under load (index lookup, not graph traversal).
    async fn check(
        &self,
        principal: PrincipalId,
        action: Action,
        resource: Resource,
    ) -> Result<bool, PermError>;

    /// Grant `role` the ability to perform `action` on `resource`.
    /// Triggers an async index rebuild for affected principals.
    async fn grant(
        &self,
        role: RoleId,
        action: Action,
        resource: Resource,
    ) -> Result<(), PermError>;

    /// Assign `principal` to `role` within `workspace`.
    async fn assign_role(
        &self,
        workspace: WorkspaceId,
        principal: PrincipalId,
        role: RoleId,
    ) -> Result<(), PermError>;
}
```

### 4.3 ReadCache (`liquid-cache`)

```rust
#[async_trait]
pub trait ReadCache: Send + Sync {
    async fn get(&self, key: ContentHash) -> Option<Bytes>;

    async fn put(&self, key: ContentHash, value: Bytes);

    /// Exact invalidation — called by ContentStore on every write.
    async fn invalidate(&self, key: ContentHash);
}
```

### 4.4 SlotBroker (`liquid-bindings`)

```rust
#[async_trait]
pub trait SlotBroker: Send + Sync {
    /// Publish a value to all subscribers of `slot` on `instance`.
    async fn publish(
        &self,
        workspace: WorkspaceId,
        instance: AppInstanceId,
        slot: SlotName,
        value: SlotValue,
    ) -> Result<(), BrokerError>;

    /// Subscribe to a slot. Returns a Stream of values.
    async fn subscribe(
        &self,
        workspace: WorkspaceId,
        instance: AppInstanceId,
        slot: SlotName,
        subscriber: PrincipalId,
    ) -> Result<BoxStream<'static, SlotValue>, BrokerError>;

    /// Wire output slot of `source_instance:source_slot` to input slot of
    /// `target_instance:target_slot`. Stored as part of the page definition.
    async fn wire(
        &self,
        workspace: WorkspaceId,
        source_instance: AppInstanceId,
        source_slot: SlotName,
        target_instance: AppInstanceId,
        target_slot: SlotName,
        wired_by: PrincipalId,
    ) -> Result<(), BrokerError>;
}
```

### 4.5 Identity (`liquid-auth`)

```rust
#[async_trait]
pub trait IdentityProvider: Send + Sync {
    /// Validate a token and return the corresponding PrincipalId.
    async fn validate_token(&self, token: &str) -> Result<PrincipalId, AuthError>;

    /// Issue a short-lived session token for `principal`.
    async fn issue_token(&self, principal: PrincipalId) -> Result<String, AuthError>;

    /// Provision a new agent principal within `workspace`.
    async fn provision_agent(
        &self,
        workspace: WorkspaceId,
        authorized_by: PrincipalId,
        name: &str,
    ) -> Result<PrincipalId, AuthError>;
}
```

---

## 5. Phase 1 — Rust Core + Flutter Shell Skeleton

**Goal:** A runnable desktop app (Linux, Windows, macOS) that can create a
workspace, open a page with a static grid, and let an agent perform a versioned
write via CLI. No app marketplace, no extensions, no multi-workspace UI.

**Duration estimate:** 12–18 months (small team of 2–4).

---

### 5.1 Milestone 1 — Rust workspace bootstrap (week 1–2)

- [ ] Create `core/Cargo.toml` workspace manifest listing all crates
- [ ] Scaffold each crate with `lib.rs`, stub types, and `#[cfg(test)]` module
- [ ] Implement `liquid-core` fully:
  - `WorkspaceId`, `AppInstanceId`, `TenantConfig`, `ComponentId`
  - `PrincipalId` (wraps a UUID; distinguishes User / Agent via enum variant)
  - `ContentHash` (SHA-256 newtype)
  - `SlotName`, `SlotValue` (typed enum: `String | Number | Bool | Json | Bytes`)
  - `StorePath` (validated UTF-8 path, workspace-relative, no `..`)
  - `Action` enum: `Read | Write | Delete | Admin`
  - `Resource` enum: `Workspace | AppInstance | Component | Page | Field`
  - `LiquidError` top-level error type re-exported by all crates
- [ ] Write unit tests for all ID types (construction, serialisation, equality)

**Success criterion:** `cargo test -p liquid-core` passes with ≥ 90% line coverage.

---

### 5.2 Milestone 2 — VCS layer (week 3–6)

- [ ] Add `jj-lib` as a dependency in `liquid-vcs`
- [ ] Implement `JujutsuContentStore` that satisfies the `ContentStore` trait:
  - One Jujutsu repo per workspace, stored under `~/.liquid/workspaces/<id>/`
  - `read` → resolves the working-copy commit, reads the file tree
  - `write` → creates a new commit on the current branch with the given author
  - `operation_log` → wraps `jj op log`
  - `undo` → wraps `jj op undo`
  - `list` → walks the tree at the given path
- [ ] Implement `InMemoryContentStore` for tests (no Jujutsu dependency)
- [ ] Pin the `jj-lib` version in `Cargo.lock`; document the pinned version in
  `docs/adr/001-jujutsu-pinning.md`

**Success criterion:** Integration test creates a workspace, writes three files,
reads them back, undoes the last write, and verifies the file is gone.

---

### 5.3 Milestone 3 — Auth + permissions (week 5–8)

- [ ] Implement `LocalIdentityProvider` in `liquid-auth`:
  - Users stored as hashed credentials in `~/.liquid/auth/users.toml`
  - Agents stored as capability tokens in `~/.liquid/auth/agents.toml`
  - Token = HMAC-SHA256-signed `{principal_id, workspace_id, expires_at}` blob
- [ ] Implement `InMemoryPermissionIndex` (stub for phase 1):
  - HashMap of `(PrincipalId, Action, Resource) → bool`
  - `check` is a HashMap lookup
  - `grant` / `assign_role` mutate the map and write through to a TOML file
- [ ] Implement RBAC role model:
  - Built-in roles: `WorkspaceOwner | WorkspaceMember | AppViewer | AppEditor | Agent`
  - Role → permission set is hard-coded in phase 1; configurable in phase 3
- [ ] Permission check is the **first** thing every `liquid-sdk-bridge` call does —
  add a `require_permission!` macro that calls `PermissionIndex::check` and
  returns `Err(LiquidError::Forbidden)` on denial

**Success criterion:** Unit test proves an agent with `AppViewer` role cannot
write; an agent with `AppEditor` role can; `WorkspaceOwner` can do both.

---

### 5.4 Milestone 4 — Cache layer stub (week 7–8)

- [ ] Implement `InProcessCache` in `liquid-cache`:
  - `Arc<DashMap<ContentHash, Bytes>>` — thread-safe, no expiry in phase 1
  - `put` stores; `get` retrieves; `invalidate` removes
- [ ] Wire into `ContentStore`: every `read` warms the cache; every `write`
  calls `invalidate` on the old hash before returning the new CommitId
- [ ] The `ReadCache` trait must be in `liquid-cache`; `InProcessCache` is
  one implementation. Phase 3 adds `RedisCache` without touching callers.

**Success criterion:** Second read of the same content hits the cache (verified
by spying on the mock `ContentStore` — the second call must not reach Jujutsu).

---

### 5.5 Milestone 5 — FFI bridge (week 8–10)

- [ ] Add `flutter_rust_bridge` to `liquid-sdk-bridge`
- [ ] Annotate the public surface of `liquid-sdk-bridge` with `#[frb]`
- [ ] Run `flutter_rust_bridge_codegen` and commit generated Dart files to
  `app/lib/bridge/` (generated files must not be manually edited)
- [ ] Expose the following initial FFI functions:
  ```rust
  pub async fn create_workspace(name: String) -> Result<WorkspaceId>;
  pub async fn list_workspaces(principal: String) -> Result<Vec<WorkspaceSummary>>;
  pub async fn load_page(workspace: WorkspaceId, page_id: PageId) -> Result<PageSnapshot>;
  pub async fn write_page(workspace: WorkspaceId, page_id: PageId,
                          snapshot: PageSnapshot, author: String,
                          message: String) -> Result<CommitId>;
  pub async fn check_permission(principal: String, action: String,
                                resource: String) -> Result<bool>;
  ```
- [ ] Write a Dart integration test that calls each function end-to-end

**Success criterion:** Dart test creates a workspace, writes a page, reads it
back, and the round-trip data matches.

---

### 5.6 Milestone 6 — Flutter shell skeleton (week 9–14)

**State management:** Riverpod. Use `AsyncNotifierProvider` for any state that
involves a Rust FFI call. UI state (hover, focus, animation) uses `StateProvider`
or local widget state. No `setState` outside of isolated leaf widgets.

- [ ] `RootShell` widget — `Row` of `ExplorerPanel` (fixed width, resizable) +
  `PageArea` (fills remaining space)
- [ ] `WorkspaceSwitcher` — compact dropdown at top of explorer; on switch,
  invalidates all workspace-scoped Riverpod providers
- [ ] `ExplorerPanel`:
  - `PageTreeView` — recursive `ListView` matching the Notion-style hierarchy
    (indent, icon, rename on double-tap, right-click context menu)
  - `AppInstanceListView` — flat list of app instances in the active workspace
  - `TagSectionView` — collapsible sections driven by tag filter rules
- [ ] `PageArea` — renders the active `PageView`
- [ ] `PageView`:
  - Contains `PageGrid`
  - Toolbar: add item, save, history, share
- [ ] `PageGrid`:
  - Fixed column/row coordinate system (configurable density, default 12 cols)
  - `GridItem` widget wraps any child (app instance or component)
  - Drag to reposition (snap to grid)
  - Resize handle on bottom-right corner of each `GridItem`
  - Maximise button expands item to fill `PageArea`
- [ ] Hard-code one placeholder `GridItem` (a coloured box) to validate the grid
  before real app instances exist

**Success criterion:** App launches on Linux. User can create a workspace, open
a page, see the grid, drag the placeholder item, and resize it.

---

### 5.7 Milestone 7 — Agent CLI (week 12–16)

See [§12 Agent CLI Specification](#12-agent-cli-specification) for the full
command grammar. Phase 1 implements the following subset:

- [ ] `liquid workspace create <name>`
- [ ] `liquid workspace list`
- [ ] `liquid page read <page-path> --workspace <id>`
- [ ] `liquid page write <page-path> --workspace <id> --file <json-file>`
- [ ] `liquid auth provision-agent <name> --workspace <id> --role <role>`
- [ ] `liquid auth token` — print a session token for the current user/agent

Authentication: token read from `LIQUID_TOKEN` env var or `~/.liquid/token`.
Every command validates the token against `IdentityProvider` before executing.
Every write command logs the commit to stdout on success.

**Success criterion:** Shell script provisions an agent, uses the token to write
a page, reads it back, and asserts the content matches.

---

### 5.8 Phase 1 Exit Criteria

- [ ] Desktop app runs on Linux, Windows, and macOS from a single `flutter build` command
- [ ] A developer can create a workspace, open a page, place a placeholder grid item
- [ ] An agent can be provisioned and perform a versioned page write via CLI
- [ ] All Rust unit tests pass; integration test suite passes
- [ ] No `unwrap()` or `expect()` in non-test Rust code — all errors propagate via `Result`
- [ ] Signed manifest verification is stubbed but the code path exists (fails open with a warning; phase 2 makes it fail closed)

---

## 6. Phase 2 — SDK + First-Party Apps

**Goal:** A real app developer can build a Liquid app with data-binding
components in a single day. The same app can be installed twice in the same
workspace with different tenant configs. An agent interacts with each instance
independently.

**Duration estimate:** 6–9 months following phase 1.

---

### 6.1 Milestone 8 — Public Dart SDK (`liquid_sdk`)

- [ ] `AppManifest` class — declarative description of an app:
  ```dart
  class AppManifest {
    final String id;           // reverse-DNS: com.example.myapp
    final String version;      // semver
    final TenantConfigSchema tenantConfigSchema;
    final List<ComponentManifest> components;
    final List<CliCommandDeclaration> cliCommands;
    final bool supportsExtensions;
    final List<Permission> requiredPermissions;
  }
  ```
- [ ] `ComponentManifest` — declares `inputSlots`, `outputSlots`,
  `minGridCells`, `maxGridCells`, and `extensionPoints` (if any)
- [ ] `LiquidComponent` abstract base class — Dart developers extend this:
  ```dart
  abstract class LiquidComponent extends StatefulWidget {
    InputSlotMap get inputs;
    OutputSlotMap get outputs;
    GridConstraints get gridConstraints;
  }
  ```
- [ ] `SlotSchema` — typed schema for a slot value (mirrors `SlotValue` in Rust)
- [ ] `GridApi` — exposes `requestResize`, `requestMaximise`
- [ ] `VcsApi` — exposes `read`, `write`, `history`, `undo` scoped to the
  current app instance
- [ ] `PermissionApi` — exposes `check(action, resource)` for the current principal
- [ ] Document each class with a one-paragraph doc comment and a usage example

**Success criterion:** A developer can create a new Flutter package, depend on
`liquid_sdk`, extend `LiquidComponent`, declare two slots, and the SDK compiles
with no errors.

---

### 6.2 Milestone 9 — Data binding broker (Rust + Dart)

- [ ] Implement `InProcessSlotBroker` in `liquid-bindings` (satisfies `SlotBroker`)
  - Uses `tokio::sync::broadcast` per slot; subscribers get their own receiver
  - `wire` stores wiring definitions in the workspace VCS as a JSON file at
    `.liquid/pages/<page_id>/bindings.json`
  - Wiring is replayed on page load — all slot subscriptions are re-established
- [ ] Expose `publish_slot`, `subscribe_slot`, `wire_slots`, `load_bindings`
  through `liquid-sdk-bridge` FFI
- [ ] In Dart SDK, `OutputSlot.emit(value)` calls `bridge.publishSlot(...)`
- [ ] In Dart SDK, `InputSlot.stream` returns a `Stream<SlotValue>` backed by
  `bridge.subscribeSlot(...)`
- [ ] Add wiring UI to `PageGrid`: long-press an output slot badge → drag to
  input slot badge → releases to call `bridge.wireSlots(...)`

**Success criterion:** Spreadsheet component emits a row-selected event; chart
component receives it and re-renders. Wiring survives page close and reopen.

---

### 6.3 Milestone 10 — Multi-instance tenant configuration

- [ ] `TenantConfigSchema` — JSON Schema (draft-07) for the configuration an
  app instance requires (e.g., API URL, credentials, data store path)
- [ ] When adding an app to a workspace, the UI presents a form generated from
  `TenantConfigSchema`; values are stored encrypted in the workspace VCS at
  `.liquid/instances/<instance_id>/tenant.enc.json`
- [ ] Each app instance has a unique `AppInstanceId` (UUID) and a user-facing
  name stored in `.liquid/instances/<instance_id>/meta.json`
- [ ] Apps receive their tenant config via `VcsApi.tenantConfig` at runtime —
  they never see configs from other instances
- [ ] Encryption: AES-256-GCM with a key derived from the workspace owner's
  password via Argon2id; key is never stored on disk

**Success criterion:** Install the same app twice, each with a different
`tenantConfig.apiUrl`. Assert that `VcsApi.tenantConfig` in instance A returns
A's URL, and in instance B returns B's URL, and neither can read the other's.

---

### 6.4 Milestone 11 — First-party reference apps

Build three apps. These exist to prove the SDK, stress-test the component
protocol, and serve as documented examples.

**TextEditor app** (`apps/text_editor/`)
- Single component: `TextEditorComponent`
- Output slot: `document:content` (type: `String`)
- Input slot: `document:initialContent` (type: `String`)
- Tenant config: `{ "storePath": "string" }` — path within workspace VCS where
  the document is persisted
- Implements: `VcsApi.write` on every save; `VcsApi.read` on mount

**Spreadsheet app** (`apps/spreadsheet/`)
- Two components: `SheetGridComponent`, `SheetFormulaBarComponent`
- Output slots: `sheet:selectedCell` (`String`), `sheet:selectedRange` (`Json`)
- Input slot: `sheet:dataSource` (`Json` — array of row objects)
- Tenant config: `{ "storePath": "string" }`

**Chart app** (`apps/chart/`)
- Single component: `ChartComponent`
- Input slot: `chart:data` (`Json` — array of `{label, value}` objects)
- Output slot: `chart:selectedSeries` (`String`)
- Tenant config: `{ "defaultChartType": "bar|line|pie" }`
- This app exists primarily to demonstrate data binding: wire
  `spreadsheet:selectedRange` → `chart:data`

**Success criterion:** User places Spreadsheet and Chart side-by-side on a page,
wires the output slot to the input slot, edits a spreadsheet cell, and the chart
updates without any manual action.

---

### 6.5 Milestone 12 — Signed manifests

- [ ] Generate an Ed25519 signing keypair for the Liquid project
  (`docs/keys/liquid-official.pub` — public key only in the repo)
- [ ] `liquid-cli` gains `manifest sign <path/to/manifest.json>` command —
  writes a `manifest.json.sig` alongside the manifest
- [ ] At app install time, `liquid-sdk-bridge` verifies the signature against
  the trusted key store before loading any app code
- [ ] Unsigned apps: rejected in production mode; allowed with a warning in
  `--dev` mode only
- [ ] Document the signing flow in `docs/sdk-guide/signing.md`

**Success criterion:** Installing an app with a tampered manifest file fails
with `LiquidError::InvalidSignature`.

---

### 6.6 Phase 2 Exit Criteria

- [ ] TextEditor, Spreadsheet, and Chart apps ship and pass their own test suites
- [ ] Data binding between Spreadsheet and Chart works end-to-end
- [ ] Same app installed twice with different tenant configs, verified to be isolated
- [ ] Signed manifest enforcement is on by default in release builds
- [ ] Agent CLI can address individual app instances:
  `liquid app <instance-name> read --workspace <id>`

---

## 7. Phase 3 — Mobile + Scale + Extensions

**Goal:** iOS and Android targets. Distributed cache and permission index replacing
phase 1 stubs. Extension API open to third-party developers.

**Duration estimate:** 6–12 months following phase 2.

---

### 7.1 Milestone 13 — Mobile targets

- [ ] Validate Flutter build tooling for iOS: `flutter build ios --release`
- [ ] Validate Flutter build tooling for Android: `flutter build appbundle`
- [ ] Audit every `GridItem` gesture for mobile:
  - Drag uses touch pan gesture (already cross-platform in Flutter, verify on device)
  - Resize handle hit area ≥ 44×44 pt (Apple HIG minimum)
  - Long-press context menus work on touch
- [ ] Explorer panel: collapses to a bottom sheet on narrow screens
  (breakpoint: `< 600 pt` width)
- [ ] Test on physical devices (not just simulators): iPhone 15 Pro, Pixel 8

**Success criterion:** All phase 1–2 features work on iOS and Android.
Frame rate ≥ 60 fps on grid interactions measured with Flutter DevTools.

---

### 7.2 Milestone 14 — Distributed cache (`liquid-cache`)

- [ ] Implement `RedisCache` satisfying `ReadCache`:
  - Uses `redis-rs` async client
  - TTL: 1 hour for page content, no TTL for immutable VCS objects
  - Key format: `liquid:ws:<workspace_id>:hash:<content_hash>`
- [ ] Connection pool configured via environment variables:
  `LIQUID_CACHE_URL`, `LIQUID_CACHE_POOL_SIZE`
- [ ] Feature flag `distributed-cache` in `Cargo.toml` — off by default;
  single-binary deployments keep `InProcessCache` with no config change
- [ ] Benchmark: warm read latency < 1 ms at p99 with 100 concurrent goroutines

**Success criterion:** Swap `InProcessCache` for `RedisCache` in integration
tests; all tests pass; no changes to application code required.

---

### 7.3 Milestone 15 — Distributed permission index

- [ ] Implement `MaterializedPermissionIndex` in `liquid-permissions`:
  - Stores the fully-expanded `(principal, action, resource) → bool` mapping
    in Redis (or any K/V store satisfying a new `IndexBackend` trait)
  - On `grant` / `assign_role`: recomputes affected rows asynchronously via
    a background Tokio task; writes are strongly consistent within 500 ms
  - On `check`: single `GET` from the index backend
- [ ] Expose index rebuild metrics (rows updated, rebuild duration) as
  Prometheus counters at `/metrics`

**Success criterion:** 20 000 concurrent `check` calls complete with p99 < 1 ms
in a load test against a real Redis instance.

---

### 7.4 Milestone 16 — Extension API

- [ ] Define `ExtensionPoint` in `liquid-core`:
  ```rust
  pub struct ExtensionPoint {
      pub name: ExtensionPointName,
      pub event: ExtensionEvent, // SlotPublished | ComponentMounted | ComponentUnmounted
      pub data_schema: SlotSchema,
  }
  ```
- [ ] Apps declare extension points in their `AppManifest`
- [ ] `ExtensionManifest` (in SDK): declares which app and which extension point
  the extension targets; must be signed before installation
- [ ] Extension runtime: when an app event fires, `liquid-bindings` checks for
  active extensions on that instance and invokes their handlers in order
- [ ] Extensions run in a restricted context: they may only call SDK APIs
  explicitly granted by the host app's extension point declaration
- [ ] Add `Extension API` section to `docs/sdk-guide/`

**Success criterion:** Write an extension that adds a word-count badge to the
`TextEditor` app using the `SlotPublished` extension point. Verify it does not
have access to the `VcsApi` (which the TextEditor did not grant).

---

### 7.5 Milestone 17 — Self-hosted registry

- [ ] Implement `liquid-registry` as a standalone Rust binary:
  - REST API: `POST /packages` (upload + verify signature), `GET /packages/<id>/<version>`
  - Storage backend: local filesystem or S3-compatible (config via env vars)
  - Signature verification at upload time; unsigned packages rejected
- [ ] `liquid-cli` gains `registry publish` and `registry install` commands
- [ ] Registry URL configurable in workspace settings (default: official registry)
- [ ] Private registry support: additional signing key(s) trusted per workspace

**Success criterion:** Publish TextEditor app to a local registry instance.
Install it in a fresh workspace from the registry. Verify signature check passes.

---

## 8. Phase 4 — Ecosystem + High Availability

**Goal:** Multi-region deployments. Community app ecosystem open. Performance
hardening against the 10 000-user/workspace scale target.

---

### 8.1 Milestone 18 — Event bus (`liquid-bindings` scale-out)

- [ ] Replace `tokio::sync::broadcast` with a Kafka-compatible event bus
  (feature-flagged: `distributed-bus`)
- [ ] Each workspace's slot events are published to topic
  `liquid.ws.<workspace_id>.slots`
- [ ] Consumers are per-replica; each replica only subscribes to workspaces
  currently active on it
- [ ] Backpressure: slow consumers are detected by lag monitoring; a consumer
  lagging > 10 000 events is dropped and must reconnect

---

### 8.2 Milestone 19 — Multi-region Jujutsu replication

- [ ] Each workspace's Jujutsu repo has one designated primary region
- [ ] Writes always go to the primary (enforced by the SDK bridge)
- [ ] Replicas receive commit notifications via the event bus and run
  `jj git fetch` to pull new commits
- [ ] Reads are served from the nearest replica's cache; on cache miss, fetch
  from the nearest replica's Jujutsu store
- [ ] RPO (recovery point objective): ≤ 1 commit (async replication, not sync)
- [ ] Document the deployment topology in `docs/ops/multi-region.md`

---

### 8.3 Milestone 20 — Scale hardening

- [ ] Load test with [k6](https://k6.io/): 10 000 concurrent users per workspace,
  sustained over 30 minutes
- [ ] Profiling targets:
  - Page load (cold): < 200 ms p99
  - Page load (warm cache): < 20 ms p99
  - Permission check: < 1 ms p99
  - Slot publish → subscriber receives: < 50 ms p99 (same region)
- [ ] Fix any regressions before opening the community registry

---

## 9. Crate Reference

### `liquid-core`
**Purpose:** Shared primitive types imported by every other crate. No I/O, no
async, no external dependencies beyond `serde` and `uuid`.

**Key types:**
- `WorkspaceId(Uuid)`, `AppInstanceId(Uuid)`, `ComponentId(Uuid)`, `PageId(Uuid)`
- `PrincipalId` — `enum { User(Uuid), Agent(Uuid) }`
- `TenantConfig(serde_json::Value)` — opaque JSON blob
- `ContentHash(String)` — hex-encoded SHA-256
- `StorePath` — validated UTF-8, workspace-relative, rejects `..`
- `SlotName(String)` — validated identifier
- `SlotValue` — `enum { Str(String) | Num(f64) | Bool(bool) | Json(Value) | Bytes(Bytes) }`
- `Action` — `enum { Read | Write | Delete | Admin }`
- `Resource` — `enum { Workspace(WorkspaceId) | AppInstance(AppInstanceId) | Component(ComponentId) | Page(PageId) | Field(String) }`
- `LiquidError` — top-level error enum re-exported by all crates

**Rules:** No `unwrap()`. Every public function returns `Result<_, LiquidError>`.

---

### `liquid-vcs`
**Purpose:** Jujutsu wrapper. Implements `ContentStore`. Owns nothing about
permissions, auth, or UI.

**Dependencies:** `liquid-core`, `jj-lib` (pinned), `liquid-cache`

**Key implementation notes:**
- One Jujutsu repo per workspace under `{data_dir}/workspaces/{workspace_id}/`
- All writes create a commit on the `main` branch with author set to
  `PrincipalId`'s string representation
- The operation log is exposed as-is from Jujutsu; no additional metadata layer
- `StorePath` maps directly to Jujutsu's file tree

---

### `liquid-auth`
**Purpose:** Identity and session management. Implements `IdentityProvider`.

**Dependencies:** `liquid-core`, `argon2`, `hmac`, `sha2`, `toml`

**Key implementation notes:**
- Phase 1: file-backed user/agent store in TOML; password hashed with Argon2id
- Phase 3: OIDC provider integration (Google, Microsoft, generic OpenID Connect)
- Token format: `{principal_id}.{workspace_id}.{expires_unix}.{hmac_hex}` — URL-safe base64 encoded
- Agents are principals with no password, only capability tokens

---

### `liquid-permissions`
**Purpose:** RBAC model and permission index. Implements `PermissionIndex`.

**Dependencies:** `liquid-core`, `liquid-auth`

**Built-in roles (phase 1, hard-coded):**

| Role | Allowed Actions |
|---|---|
| `WorkspaceOwner` | All actions on all resources in workspace |
| `WorkspaceMember` | Read all; Write own pages and app instances |
| `AppViewer` | Read a specific app instance |
| `AppEditor` | Read + Write a specific app instance |
| `Agent` | Configured per-agent; cannot exceed authorising principal |

**Index storage (phase 1):** TOML file at `{data_dir}/workspaces/{id}/permissions.toml`
**Index storage (phase 3):** Redis key-value store

---

### `liquid-cache`
**Purpose:** Content-addressable read cache. Implements `ReadCache`.

**Dependencies:** `liquid-core`

**Phase 1:** `InProcessCache` (`Arc<DashMap<ContentHash, Bytes>>`)
**Phase 3:** `RedisCache` (feature-flagged `distributed-cache`)

**Cache warming:** `liquid-vcs` calls `cache.put(hash, bytes)` on every read
miss. `ContentHash` is computed from the content bytes before storing.

---

### `liquid-bindings`
**Purpose:** Data binding pub/sub broker. Implements `SlotBroker`.

**Dependencies:** `liquid-core`, `liquid-vcs` (for persisting wiring definitions),
`liquid-permissions` (checks subscriber has read permission on source slot)

**Wiring persistence:** stored at `.liquid/pages/{page_id}/bindings.json`
within the workspace VCS. Loaded on page open; re-establishes all subscriptions.

---

### `liquid-sdk-bridge`
**Purpose:** FFI surface. Thin adapter layer only — no business logic.

**Dependencies:** All other crates, `flutter_rust_bridge`

**Rules:**
- Every function checks permission before doing anything else
- Every function is `async`
- Return types must be serialisable to Dart via `flutter_rust_bridge`
- Generated Dart files go to `app/lib/bridge/` — commit them, never edit manually

---

### `liquid-cli`
**Purpose:** `liquid` binary for agent interactions.

**Dependencies:** `liquid-sdk-bridge` (reuses the same Rust logic, not the FFI layer), `clap`

**Authentication:** `LIQUID_TOKEN` env var → validated by `IdentityProvider` on
every command.

See [§12 Agent CLI Specification](#12-agent-cli-specification).

---

## 10. Flutter Application Reference

### State management

Use **Riverpod** throughout. Rules:
- One provider per remote resource (page, workspace list, app instance list)
- Providers are `AsyncNotifierProvider<T>` when backed by a Rust FFI call
- UI state (hover, focus, animation progress) uses local `StatefulWidget` or
  `StateProvider` — never a full `AsyncNotifier`
- Providers are invalidated at workspace switch by calling
  `ref.invalidate(workspaceProvider)` and cascading

### Folder conventions

| Folder | Contains |
|---|---|
| `shell/` | `RootShell`, `WorkspaceSwitcher`, top-level layout |
| `explorer/` | `ExplorerPanel`, `PageTreeView`, `AppInstanceListView`, `TagSectionView` |
| `grid/` | `PageGrid`, `GridItem`, `GridResizeHandle`, `GridDropTarget` |
| `pages/` | `PageView`, `PageToolbar`, page model DTOs |
| `bindings/` | `SlotWiringOverlay`, slot badge widgets, `BindingEditorSheet` |
| `state/` | All Riverpod providers and notifiers |
| `bridge/` | Generated FFI bindings — do not touch |

### Widget naming conventions

- Screens / full-page views: `XxxView`
- Reusable widgets: `XxxWidget` or plain `Xxx`
- Providers: `xxxProvider`
- Notifiers: `XxxNotifier`

### Grid implementation notes

The grid uses Flutter's `CustomMultiChildLayout` with a `GridLayoutDelegate`.
Each `GridItem` holds a `GridPlacement` (col, row, colSpan, rowSpan).
Drag-to-reposition uses `Draggable` + `DragTarget` with snapping logic in the
delegate. Resize uses a `GestureDetector` on the bottom-right corner computing
new span from delta position.

---

## 11. SDK Design Specification

### App manifest (Dart)

```dart
@immutable
class AppManifest {
  final String id;             // e.g. "com.example.crm"
  final String version;        // semver "1.2.3"
  final TenantConfigSchema tenantConfigSchema;
  final List<ComponentManifest> components;
  final List<CliCommandDeclaration> cliCommands;
  final bool supportsExtensions;
  final List<ExtensionPoint> extensionPoints; // empty if !supportsExtensions
  final List<PermissionRequest> requiredPermissions;
}
```

### Component protocol

```dart
abstract class LiquidComponent extends StatefulWidget {
  /// Unique identifier within the app. Matches ComponentManifest.id.
  String get componentId;

  /// Input slots this component subscribes to.
  /// Keys are SlotName strings; values describe the expected type.
  Map<String, SlotSchema> get inputSlots;

  /// Output slots this component publishes to.
  Map<String, SlotSchema> get outputSlots;

  /// Grid size constraints.
  GridConstraints get gridConstraints;

  /// Called by the runtime when an input slot receives a new value.
  void onSlotValue(String slotName, SlotValue value);
}
```

The runtime — not the component — owns slot subscriptions. A component never
calls `bridge.subscribeSlot` directly; it receives values via `onSlotValue`.
A component emits values by calling `context.emitSlot(slotName, value)`, which
the runtime forwards to `bridge.publishSlot`.

### Tenant config schema

`TenantConfigSchema` wraps a JSON Schema (draft-07) object. The install UI
auto-generates a form from it. Sensitive fields (passwords, tokens) are declared
with `"x-liquid-sensitive": true`; the runtime encrypts them before storage.

### Versioning

SDK versions follow semver. The `AppManifest.sdkVersion` field declares the
minimum SDK version required. The runtime rejects apps requiring a newer SDK
than is installed, with a clear error message.

---

## 12. Agent CLI Specification

### Authentication

Every command requires a valid token, provided via:
1. `LIQUID_TOKEN` environment variable (preferred for automation)
2. `~/.liquid/token` file (written by `liquid auth login`)
3. `--token <value>` flag (not recommended — visible in process list)

### Command grammar

```
liquid <resource> <verb> [args] [flags]

Global flags:
  --workspace <id>    target workspace (required for most commands)
  --as <agent-name>   run as a named agent (requires matching token)
  --format json|text  output format (default: text)

liquid workspace create <name>
liquid workspace list
liquid workspace delete <id>

liquid page read <page-path>
liquid page write <page-path> --data <json>
liquid page history <page-path>
liquid page undo <page-path> --op <operation-id>

liquid app list
liquid app install <app-id>@<version> --name <instance-name> \
  --tenant-config <json-file>
liquid app uninstall <instance-name>

liquid app <instance-name> read <component-name>/<field>
liquid app <instance-name> write <component-name>/<field> --data <json>
liquid app <instance-name> slot subscribe <slot-name>   # streams events to stdout
liquid app <instance-name> slot publish <slot-name> --data <json>

liquid auth login                        # interactive; writes ~/.liquid/token
liquid auth provision-agent <name> \
  --role <role> [--expires <duration>]   # prints agent token
liquid auth token                        # print current token (for scripting)
liquid auth whoami                       # print current principal info
```

### Output format

`--format json` outputs newline-delimited JSON objects. Every response object
has at minimum `{ "ok": true|false, "data": <payload> | null, "error": null | "..." }`.

`--format text` (default) outputs human-readable lines. Errors go to stderr
with a non-zero exit code.

---

## 13. Data Binding Protocol

### Slot types

| Type | Dart | Rust | Notes |
|---|---|---|---|
| `String` | `String` | `String` | UTF-8 |
| `Number` | `double` | `f64` | JSON number |
| `Bool` | `bool` | `bool` | |
| `Json` | `Map<String, dynamic>` | `serde_json::Value` | arbitrary JSON object |
| `Bytes` | `Uint8List` | `bytes::Bytes` | binary; zero-copy via FFI |

### Slot naming

Slot names follow the pattern `<namespace>:<descriptor>`, e.g.
`sheet:selectedRange`, `chart:data`. Namespaces are declared in the
`ComponentManifest`. Cross-app binding is allowed if both components share a
compatible slot schema.

### Schema compatibility

Two slots are compatible if their `SlotSchema` types match exactly. The runtime
refuses to wire incompatible slots and shows a clear error in the wiring UI.
Schema versioning: if an app updates a slot's type in a new version, it must
maintain a `v1`-named slot alongside the new `v2` slot for one major version
before removing the old one.

### Wiring persistence format

`.liquid/pages/{page_id}/bindings.json`:
```json
{
  "version": 1,
  "wires": [
    {
      "source": { "instanceId": "...", "slot": "sheet:selectedRange" },
      "target": { "instanceId": "...", "slot": "chart:data" },
      "wiredBy": "principal:...",
      "wiredAt": "2026-05-03T12:00:00Z"
    }
  ]
}
```

---

## 14. Testing Strategy

### Rust

- **Unit tests** (`#[test]` inside each crate): test pure logic, types, and
  in-memory implementations. Run with `cargo test -p <crate>`.
- **Integration tests** (`core/tests/`): cross-crate scenarios using real
  Jujutsu repos in a temp directory. Run with `cargo test --test integration`.
- **No mocks for core interfaces** — use the provided in-memory implementations
  (`InMemoryContentStore`, `InProcessCache`, `InMemoryPermissionIndex`).
  These are first-class code, not test scaffolding.
- Coverage target: ≥ 80% line coverage on all crates except `liquid-cli`.

### Dart / Flutter

- **Unit tests** (`test/unit/`): test Riverpod providers with mocked FFI bridge.
  The bridge interface must have a `MockBridge` implementation committed to the
  test directory.
- **Widget tests** (`test/widget/`): test individual widgets in isolation using
  `flutter_test`.
- **Integration tests** (`integration_test/`): full app on a real device or
  emulator using `flutter_driver`. Covers the critical path:
  create workspace → open page → place grid item → wire slots → verify binding.

### Agent CLI

- Shell-based integration tests using `bats` (Bash Automated Testing System)
- Tests provision an agent, execute CLI commands, assert output via `jq`
- Run as part of CI on every push to `main`

### CI pipeline (suggested)

```yaml
jobs:
  rust:
    - cargo fmt --check
    - cargo clippy -- -D warnings
    - cargo test --workspace
  flutter:
    - dart format --output=none --set-exit-if-changed .
    - flutter analyze
    - flutter test
  cli:
    - bats tests/cli/
```

---

## 15. Key Design Decisions

These decisions are final for their respective phases. An agent must not reverse
them without creating a new ADR in `docs/adr/`.

### ADR-001 — Jujutsu over Git for VCS storage
**Decision:** Use Jujutsu as the storage layer.
**Rationale:** Jujutsu's operation log provides undo of any operation (not just
commits), first-class conflict resolution, and a cleaner API than libgit2.
The cost is API instability in `jj-lib`; mitigated by pinning and tracking upstream.
**Consequence:** Do not expose Git-specific concepts (branches as refs, staging area)
in the `ContentStore` interface. The interface must be VCS-agnostic.

### ADR-002 — Flutter/Dart as the universal UI layer
**Decision:** Flutter for all five platforms; no WebView.
**Rationale:** Impeller GPU renderer, single codebase, proven at Notion-quality
complexity (AppFlowy). The WebView alternative introduces a performance ceiling
that compounds with app complexity, as Notion's migration history demonstrates.
**Consequence:** All UI code is Dart. No HTML/CSS/JS in the rendering pipeline.

### ADR-003 — Tenant config is app-instance-level, not workspace-level
**Decision:** Tenant config belongs to an app instance, not the workspace.
**Rationale:** The same app can be installed multiple times in one workspace
with different data sources (e.g., CRM-US and CRM-EU). Making tenant a
workspace concept would prevent this.
**Consequence:** `WorkspaceId` does not imply a single data source.
Per-instance tenant config must be encrypted and stored per-instance.

### ADR-004 — Permission checks happen in `liquid-sdk-bridge`, nowhere else
**Decision:** Every FFI function in `liquid-sdk-bridge` begins with a permission check.
**Rationale:** A single enforcement point is auditable. Distributed enforcement
(each crate checks its own permissions) leads to gaps.
**Consequence:** `liquid-vcs`, `liquid-bindings`, etc. do **not** check permissions.
They trust that the bridge has already validated the call. This is only safe
because Dart code cannot call these crates directly — only through the bridge.

### ADR-005 — Storage interface is abstract from the first line
**Decision:** `ContentStore`, `ReadCache`, and `PermissionIndex` are traits.
Phase 1 ships in-process stubs; phase 3 swaps in distributed implementations.
**Rationale:** Retrofitting abstraction across storage callsites costs more than
the initial discipline.
**Consequence:** Application code never imports `JujutsuContentStore`,
`InProcessCache`, or `InMemoryPermissionIndex` directly. Dependency injection
at startup only.

### ADR-006 — Components communicate only through data slots
**Decision:** Components may not hold direct Dart references to other components.
All cross-component communication goes through the `SlotBroker`.
**Rationale:** Direct references couple components, violate the cross-app
compatibility contract, and create security gaps (one component reading another's
internal state).
**Consequence:** The component runtime must enforce this — there is no legitimate
reason for a component widget to receive another component widget as a constructor
argument.
