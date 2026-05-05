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
16. [Release Process](#16-release-process)

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

**App developers never import platform-specific APIs.**
`liquid_sdk` is the only interface to platform capabilities. If a developer
building a Liquid app reaches for `dart:io`, `path_provider`, `url_launcher`,
or any platform channel, that is a signal the SDK is missing an abstraction —
not a reason to work around it. Add the missing API to `liquid_sdk` first.
An app that imports platform-specific code cannot guarantee cross-platform
availability and must be rejected by the registry CI.

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

All methods return [`liquid_core::Result<T>`][result] (i.e. `Result<T,
LiquidError>`). The earlier draft specified a domain-specific `StoreError`,
but the workspace-wide policy in `CLAUDE.md` is that every public function
returns `Result<_, LiquidError>` — so the `liquid-vcs` crate normalises to
that single error type and does not introduce a parallel hierarchy.

[result]: https://github.com/open-equilibrium/liquid/blob/main/core/liquid-core/src/error.rs

```rust
#[async_trait]
pub trait ContentStore: Send + Sync {
    /// Read the current content of `path` in `workspace`.
    /// `LiquidError::NotFound` if the workspace or path does not exist.
    async fn read(
        &self,
        workspace: WorkspaceId,
        path: &StorePath,
    ) -> Result<Bytes>;

    /// Atomically write `content` to `path`, recording an operation
    /// attributed to `author` with `message`. Returns the new `CommitId`.
    async fn write(
        &self,
        workspace: WorkspaceId,
        path: &StorePath,
        content: Bytes,
        author: PrincipalId,
        message: &str,
    ) -> Result<CommitId>;

    /// Return up to `limit` operation log entries for `workspace`,
    /// newest first.
    async fn operation_log(
        &self,
        workspace: WorkspaceId,
        limit: usize,
    ) -> Result<Vec<Operation>>;

    /// Invert the operation identified by `op_id`. Returns the new
    /// `CommitId` for the synthetic commit that captures the inversion.
    async fn undo(
        &self,
        workspace: WorkspaceId,
        op_id: OperationId,
    ) -> Result<CommitId>;

    /// List paths beneath `prefix` (directory-style listing).
    async fn list(
        &self,
        workspace: WorkspaceId,
        prefix: &StorePath,
    ) -> Result<Vec<StorePath>>;
}
```

`Operation` is a `liquid-vcs` type carrying `{id, commit, timestamp,
author, message, kind}`, where `kind` is `Create | Update | Delete | Undo`
and each variant captures enough state (e.g. `prev: Bytes` on `Update`) to
invert the operation without consulting the underlying store.

### 4.2 PermissionIndex (`liquid-permissions`)

All methods return [`liquid_core::Result<T>`][result]. Per the §4.1
convention, errors normalise to `LiquidError` rather than a parallel
`PermError` hierarchy — same reasoning, same workspace-wide policy
(`CLAUDE.md`).

The `RoleId` parameter from the original draft is replaced by a
[`BuiltInRole`] enum because Phase 1 hard-codes the role → permission
matrix (§9). The `grant(role, action, resource)` method from the draft is
deferred to Phase 3, when custom roles become configurable; in Phase 1 it
would be unreachable code, so it is omitted rather than stubbed.

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
    ) -> Result<bool>;

    /// Bind `principal` to `role` within `workspace`. For roles whose
    /// `requires_scope()` is true (`AppViewer`, `AppEditor`), `scope`
    /// must be `Some(_)`; for workspace-wide roles, `scope` may be `None`.
    async fn assign_role(
        &self,
        workspace: WorkspaceId,
        principal: PrincipalId,
        role: BuiltInRole,
        scope: Option<Resource>,
    ) -> Result<()>;

    /// Reverse `assign_role`. Idempotent.
    async fn revoke_role(
        &self,
        workspace: WorkspaceId,
        principal: PrincipalId,
        role: BuiltInRole,
        scope: Option<Resource>,
    ) -> Result<()>;
}
```

The canonical permission gate at every bridge / CLI callsite is the
`require_permission!(index, principal, action, resource)` macro
(re-exported from `liquid_permissions`). It awaits `check` and returns
`Err(LiquidError::Forbidden)` from the enclosing `async fn` on denial.

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

All methods return [`liquid_core::Result<T>`][result] (same `LiquidError`
normalisation as §4.1 / §4.2). The original draft's `AuthError` is folded
into `LiquidError::Forbidden` (auth failure) and `LiquidError::InvalidInput`
(malformed token / bad input) — never leak which mode of failure occurred
to the caller.

```rust
#[async_trait]
pub trait IdentityProvider: Send + Sync {
    /// Validate a token and return the corresponding PrincipalId.
    /// Returns `LiquidError::Forbidden` for any failure mode.
    async fn validate_token(&self, token: &str) -> Result<PrincipalId>;

    /// Issue a short-lived session token for `principal`.
    async fn issue_token(&self, principal: PrincipalId) -> Result<String>;

    /// Provision a new agent principal within `workspace`. The bridge
    /// layer is responsible for permission-gating this call.
    async fn provision_agent(
        &self,
        workspace: WorkspaceId,
        authorized_by: PrincipalId,
        name: &str,
    ) -> Result<PrincipalId>;
}
```

**Token format (Phase 1).** `principal . expires_unix . hmac_hex` —
three dot-separated fields, each URL-safe by construction. The
`workspace_id` field from the original §9 draft is dropped: a session
token represents the principal's identity, not their authority over a
specific workspace; authority comes from `PermissionIndex` bindings.
Carrying the field would invite the bug of misinterpreting it as
authorisation. `principal` is encoded as `u:<uuid>` for users or
`a:<uuid>` for agents.

**Local backend.** `LocalIdentityProvider` (Phase 1) persists users at
`<root>/users.toml` (Argon2id-hashed passwords) and provisioned agents
at `<root>/agents.toml`. It exposes two inherent helpers beyond the
trait surface — `register_user(username, password)` and
`authenticate_user(username, password) -> token` — that are local-only;
Phase 3's OIDC backend will replace the `authenticate_user` flow with a
browser redirect and code exchange instead of password verification.

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

ADR-001 splits this milestone into two tasks: ship a durable backend
*now* (filesystem) and defer the `jj-lib` integration to its own task.
Both implementations live behind the same `ContentStore` trait, so
application code does not change when the swap happens.

- [x] Implement `InMemoryContentStore` (TASK-002) — test/dev backend,
      no persistence. Satisfies the trait without any Jujutsu dependency.
- [x] Implement `FilesystemContentStore` (TASK-003) — durable on-disk
      backend used in Phase 1. Layout per ADR-001:
      ```
      <root>/<workspace_id>/
        files/<store_path>     # raw bytes; tmp-then-rename atomic write
        op_log.jsonl           # newline-delimited Operation JSON
      ```
- [x] Accept ADR-001 documenting the deferral.
- [ ] Implement `JujutsuContentStore` (TASK-004) — thin wrapper over a
      pinned `jj-lib` version, satisfying the same trait. The
      integration tests written against `FilesystemContentStore` apply
      unchanged to the new impl. Pinning policy: exact patch in
      `Cargo.lock`; Renovate `jj-lib` rule blocks auto-upgrade.

**Success criterion:** Integration test creates a workspace, writes
three files, reads them back, undoes the last write, and verifies the
file is gone — passes against both `FilesystemContentStore` (today) and
`JujutsuContentStore` (when TASK-004 lands). On-disk durability is
proven by re-opening the same root in a fresh process and reading the
data back.

---

### 5.3 Milestone 3 — Auth + permissions (week 5–8)

The trait shapes here reflect ADR-002 (M3 trait scoping): in-memory
backends ship now, disk-backed variants are deferred, and the original
§4.2 / §4.5 drafts are simplified to drop Phase-3-only surface
(`grant`, `RoleId`, workspace-bound tokens).

- [x] Implement `LocalIdentityProvider` in `liquid-auth` (TASK-006):
  - Users stored as hashed credentials in `<root>/users.toml` (Argon2id
    via the `argon2` crate; never the raw password)
  - Agents stored in `<root>/agents.toml` with id, name, workspace,
    authorising principal, and creation time
  - Atomic writes (tmp-then-rename), same idiom as
    `FilesystemContentStore`
  - Session token: `principal . expires_unix . hmac_hex`
    (HMAC-SHA256, three dot-separated URL-safe-by-construction fields).
    `principal` is `u:<uuid>` for users, `a:<uuid>` for agents.
    No `workspace_id` field — see ADR-002.
  - All auth failure modes collapse to `LiquidError::Forbidden`; never
    leak which mode failed
- [x] Implement `InMemoryPermissionIndex` in `liquid-permissions`
      (TASK-005):
  - `HashSet<Binding>` where
    `Binding = { workspace, principal, role, scope }`
  - `check` is a single pass over the principal's bindings; the
    role → permission matrix is encoded in `BuiltInRole::permits`
  - `assign_role` validates that scope-required roles
    (`AppViewer`/`AppEditor`) carry `Some(Resource)`
  - `revoke_role` is idempotent
- [x] Implement RBAC role model (`BuiltInRole`):
  - Built-in roles:
    `WorkspaceOwner | WorkspaceMember | AppViewer | AppEditor | Agent`
  - Role → permission set hard-coded in `BuiltInRole::permits`;
    runtime-configurable role grants are deferred to Phase 3, when the
    `grant(role, action, resource)` method returns to the trait
- [x] Permission check is the **first** thing every `liquid-sdk-bridge`
      call does — `require_permission!(index, principal, action,
      resource)` macro calls `PermissionIndex::check` and returns
      `Err(LiquidError::Forbidden)` on denial. Re-exported from
      `liquid_permissions`.
- [ ] Implement disk-backed `PermissionIndex` (TASK-007) — TOML file at
      `<root>/workspaces/<id>/permissions.toml` per §9. The trait is
      already in place; this is purely an additional implementation.

**Success criterion (proven in
`core/liquid-permissions/tests/m3_end_to_end.rs`):** Unit test wires
`liquid-auth` and `liquid-permissions` together along the path a real
bridge call would follow (issue token → validate token →
`require_permission!`) and proves an agent with `AppViewer` role cannot
write, an agent with `AppEditor` role can, and `WorkspaceOwner` can do
both.

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
- [ ] Activate the `redis` service in `docker-compose.yml` (`just services-up phase3`)
  for local development; document in `docs/ops/local-dev.md`

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
- [ ] Activate the `redpanda` service in `docker-compose.yml`
  (`just services-up phase4`) for local development

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
**Purpose:** Versioned content store. Owns nothing about permissions,
auth, or UI. Per ADR-001, ships two `ContentStore` implementations
behind a single trait.

**Dependencies:** `liquid-core`, `async-trait`, `bytes`, `serde`,
`serde_json`. `jj-lib` (pinned) is added by TASK-004.

**Implementations (Phase 1):**

| Impl | Status | When to use |
|---|---|---|
| `InMemoryContentStore` | Shipped (TASK-002) | Tests, dev mode |
| `FilesystemContentStore` | Shipped (TASK-003) | Default Phase-1 backend; durable |
| `JujutsuContentStore` | Planned (TASK-004) | Replaces `FilesystemContentStore` once `jj-lib` is pinned |

**`FilesystemContentStore` layout (per ADR-001):**

```
<root>/<workspace_id>/
  files/<store_path>     # raw bytes; tmp-then-rename atomic write
  op_log.jsonl           # append-only newline-delimited Operation JSON
```

`StorePath` maps directly into the `files/` subtree. Operation log is
parsed by re-reading the whole file on every `operation_log` / `undo`
call — fine for Phase 1; Phase 2+ may add an in-memory cache or binary
log if it becomes a hot path.

**`JujutsuContentStore` notes (TASK-004):**
- One Jujutsu repo per workspace under `{data_dir}/workspaces/{workspace_id}/`
- All writes create a commit on the `main` branch with author set to
  `PrincipalId`'s string representation
- Operation log exposed as-is from Jujutsu; no additional metadata layer
- `StorePath` maps directly to Jujutsu's file tree
- `jj-lib` is pinned to an exact patch version; Renovate's `jj-lib`
  rule blocks auto-upgrade and routes bumps to manual review

---

### `liquid-auth`
**Purpose:** Identity and session management. Implements `IdentityProvider`.

**Dependencies:** `liquid-core`, `argon2`, `hmac`, `sha2`, `toml`,
`hex`, `uuid`, `async-trait`, `serde`.

**Key implementation notes (Phase 1, per ADR-002):**
- File-backed user/agent store in TOML at `<root>/users.toml` and
  `<root>/agents.toml`; atomic writes (tmp-then-rename)
- Passwords hashed with Argon2id via the `argon2` crate; raw passwords
  never persisted
- Token format: `principal . expires_unix . hmac_hex` — three
  dot-separated, URL-safe-by-construction fields, signed with
  HMAC-SHA256. `principal` is `u:<uuid>` for users, `a:<uuid>` for
  agents. The `workspace_id` field from the original draft is dropped:
  a token is identity, not authority.
- Auth-failure modes (tampered, expired, unknown signing key, malformed,
  unknown user, wrong password) all collapse to
  `LiquidError::Forbidden` — never leak which mode failed
- Agents are principals with no password, only capability tokens
- Trait surface is minimal (`validate_token`, `issue_token`,
  `provision_agent`); local-only helpers (`register_user`,
  `authenticate_user`) are inherent methods on
  `LocalIdentityProvider`. Phase 3's OIDC backend will replace the
  `authenticate_user` flow with a browser redirect + code exchange.

---

### `liquid-permissions`
**Purpose:** RBAC model and permission index. Implements `PermissionIndex`.

**Dependencies:** `liquid-core`, `async-trait`, `serde`. (No runtime
dependency on `liquid-auth`; `PrincipalId` lives in `liquid-core` and
the M3 end-to-end test in this crate uses `liquid-auth` only as a
dev-dependency.)

**Built-in roles (Phase 1, hard-coded in `BuiltInRole::permits` per
ADR-002):**

| Role | Allowed Actions | Scope at assignment |
|---|---|---|
| `WorkspaceOwner` | All actions on all resources in workspace | `None` |
| `WorkspaceMember` | Read all; Write/Delete pages, app instances, components; no Admin | `None` |
| `AppViewer` | Read on the scoped app instance / component | `Some(Resource::AppInstance(_))` |
| `AppEditor` | Read + Write on the scoped app instance / component | `Some(Resource::AppInstance(_))` |
| `Agent` | Marker only; grants nothing on its own — agents derive authority from additional role bindings (cannot exceed authorising principal) | either |

`BuiltInRole::requires_scope()` enforces that `AppViewer` and
`AppEditor` are assigned with a non-`None` scope; the index returns
`LiquidError::InvalidInput` otherwise.

**Permission gate.** `require_permission!(index, principal, action,
resource)` is the canonical macro at every bridge / CLI callsite
(CLAUDE.md rule 4). It awaits `PermissionIndex::check` and returns
`Err(LiquidError::Forbidden)` from the enclosing `async fn` on denial.

**Implementations:**

| Impl | Status | When to use |
|---|---|---|
| `InMemoryPermissionIndex` | Shipped (TASK-005) | Tests, Phase-1 dev mode |
| TOML-backed `PermissionIndex` | Planned (TASK-007) | Phase-1 production; persists to `<root>/workspaces/<id>/permissions.toml` |
| Redis-backed `PermissionIndex` | Phase 3 | Distributed deployments |

**Phase 3 trait extensions (deferred per ADR-002).** `grant(role,
action, resource)` returns to the trait when custom roles ship; a
`Role::Custom(RoleId)` variant joins `BuiltInRole`. Existing call sites
remain valid — the change is additive.

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

### Platform Abstraction Contract

The single most important developer-facing guarantee: **write one Dart package,
run unchanged on Linux, Windows, macOS, iOS, and Android.** This guarantee is
only possible if `liquid_sdk` is the complete interface to all platform
capabilities.

**Allowed imports in an app package:**

| Allowed | Prohibited |
|---|---|
| `liquid_sdk` | `dart:io` |
| `dart:core`, `dart:math`, `dart:convert` | `dart:html` |
| `dart:typed_data`, `dart:async` | Any Flutter plugin (`path_provider`, `url_launcher`, `camera`, …) |
| Pure Dart packages with no platform code | Any `MethodChannel` or platform channel |

**`liquid_sdk` must cover every platform capability an app legitimately needs:**

| App need | SDK API |
|---|---|
| Persistent storage | `VcsApi.read` / `VcsApi.write` |
| Tenant-scoped config | `VcsApi.tenantConfig` |
| Permission checks | `PermissionApi.check` |
| Cross-component data | `InputSlot` / `OutputSlot` |
| Notifications (phase 2) | `NotificationApi.send` |
| Content sharing (phase 3) | `ShareApi.share` |
| Deep links / navigation (phase 3) | `DeeplinkApi.register` |

If a developer cannot accomplish a legitimate goal without a platform import,
the SDK is incomplete. Open an SDK issue; do not bypass the abstraction.

**Enforcement:**

1. `sdk/liquid_sdk/analysis_options.yaml` includes a custom lint rule
   `no_platform_imports` (implemented in `sdk/liquid_sdk_lint/`) that makes
   importing banned packages an analyzer error.
2. The registry CI pipeline runs `flutter build <target>` for all five targets
   on every package upload. A package that fails any target is rejected with the
   failing target named in the error.
3. Reference apps (`apps/`) serve as the acceptance test: if they compile for
   all five targets without platform imports, the contract holds.

### SDK Performance Contract

App developers must not implement their own caches or assume lower latency
than these bounds. If a bound is exceeded, that is a platform regression.

| API call | Expected latency | Condition |
|---|---|---|
| `VcsApi.read(path)` | < 20 ms p99 | Warm cache hit |
| `VcsApi.read(path)` | < 200 ms p99 | Cold cache (Jujutsu read) |
| `VcsApi.write(path, content)` | < 100 ms p99 | Single-file commit |
| `VcsApi.tenantConfig` | < 1 ms | Loaded at mount, held in memory |
| `PermissionApi.check(action, resource)` | < 1 ms p99 | Materialized index lookup |
| `OutputSlot.emit(value)` | < 5 ms p99 | In-process broker (phase 1–2) |
| `InputSlot.stream` first event | < 50 ms p99 | Same region (phase 4) |

These targets apply per workspace. They are validated in Milestone 20 load tests.
An app receiving slot events faster than < 50 ms p99 is a bonus, not a contract.

### Component Isolation Enforcement

ADR-006 prohibits direct Dart references between components. The following
mechanisms make this enforceable without Dart isolates (which are rejected
because they prohibit shared Flutter widget trees and require serialising
every render update):

**1 — Widget tree scoping.**
`PageGrid` renders each `GridItem` with a fresh `LiquidComponentScope`
`InheritedWidget` at its root. This scope holds the `ComponentContext` for that
component only. `context.emitSlot` and `context.readSlot` are methods on
`LiquidComponentContext` — they only exist in that component's widget subtree.

**2 — No sibling access.**
Flutter's `BuildContext.findAncestorWidgetOfExactType` walks only up the widget
tree. Components are siblings under `PageGrid`, not ancestors of each other.
A component cannot reach a `LiquidComponentScope` that is not its own ancestor.

**3 — Static lint rule (`no_cross_component_reference`).**
Shipped in `sdk/liquid_sdk_lint/`. Flags any `LiquidComponent` subclass field
that is typed as another `LiquidComponent` subclass. Applied in the SDK's own
`analysis_options.yaml` and recommended for all app packages. Applied as an
error in registry CI.

**4 — No shared API surface.**
`liquid_sdk` exports no function that takes a `ComponentId` of a foreign
component as an argument. The only way to address another component is via a
named slot — which goes through the `SlotBroker` permission check in Rust.

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

### App CLI surface design guidance

App developers declare CLI commands in `AppManifest.cliCommands`. These are
auto-generated as subcommands of `liquid app <instance-name>`. Rules for
designing a good CLI surface:

**Model commands around data, not UI actions.**
`read component/pipeline-sheet --row 0` is good.
`click-button add-row` is not a CLI command.

**Every readable state must be readable via CLI.**
If an agent cannot observe a component's current state via `liquid app ...
read`, the app is not agent-friendly. Design the `read` handler first.

**Every writable state must be writable via CLI.**
The agent must be able to perform the same mutations as a human user.
`liquid app <name> write component/<name> --data <json>` is the primary
mutation surface.

**Slot subscribe is the agent's event stream.**
`liquid app <name> slot subscribe <slot-name>` streams newline-delimited JSON
events to stdout indefinitely. Agents use this to react to changes. Every
significant state change should be published to a slot.

**`CliCommandDeclaration` schema:**
```dart
@immutable
class CliCommandDeclaration {
  final String verb;          // e.g. "read", "write", "export"
  final String target;        // e.g. "component/pipeline-sheet"
  final String description;   // shown in --help
  final List<CliFlag> flags;  // --row, --format, etc.
  final PermissionRequest requiredPermission; // checked before execution
}
```

The Liquid runtime auto-generates `--help` output from these declarations.
App developers do not write a CLI binary; the manifest is sufficient.

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
- **Integration / E2E tests** (`integration_test/`): use **`patrol`** as the
  Flutter E2E framework (`flutter pub add patrol --dev`). `patrol` wraps
  `integration_test` with ergonomic interaction APIs (`tester.tap`,
  `tester.scroll`, `tester.longPress`) and first-class support for iOS/Android
  native interactions. Covers the critical path: create workspace → open page →
  place grid item → wire slots → verify binding.
  - Add `patrol` to `app/pubspec.yaml` at Milestone 6 (Flutter shell skeleton)
  - Add `patrol_cli` globally: `dart pub global activate patrol_cli`
  - Run: `patrol test` (desktop) or `patrol test --device <id>` (mobile)

### Agent CLI

- Shell-based integration tests using `bats` (Bash Automated Testing System)
- Tests provision an agent, execute CLI commands, assert output via `jq`
- Run as part of CI on every push to `main`

### CI pipeline

Defined in `.github/workflows/ci.yml`. A leading `detect` job inspects the
working tree for each layer's marker file (`core/Cargo.toml`,
`sdk/liquid_sdk/pubspec.yaml`, `app/pubspec.yaml`,
`apps/text_editor/pubspec.yaml`, `tests/cli/*`) and exposes one boolean
output per layer; every other job declares `needs: detect` and gates on the
matching output. Layers whose code does not exist yet show as skipped
rather than failing — `hashFiles()` is not used at job level because
GitHub Actions rejects it there. Locally, use `just check` (runs `just
lint` + `just test`) to replicate the full CI suite before pushing.

```
detect              — outputs rust|sdk|app|apps|cli = "true"|"false"
rust                — fmt + clippy + tests  (Linux / Windows / macOS)
sdk                 — dart format + analyze + flutter test --coverage
app                 — dart format + analyze + flutter test --coverage + flutter build <5 targets>
apps-platform-check — flutter analyze + linux build per reference app (ADR-008)
cli                 — bats tests/cli/
```

Coverage reports (tarpaulin for Rust, lcov for Flutter) are uploaded to Codecov
with per-layer flags (`rust`, `sdk`, `app`). Set the `CODECOV_TOKEN` secret in
GitHub → Settings → Secrets to enable upload.

---

## 15. Key Design Decisions

These decisions are final for their respective phases. An agent must not reverse
them without creating a new ADR in `docs/adr/`.

> **Numbering note.** §15's ADR-NNN labels are inline summaries of the
> *strategic* decisions baked into this plan. Tactical ADRs created by
> implementers live as separate files in `docs/adr/NNN-title.md` with
> their own numbering sequence (currently
> `docs/adr/001-jujutsu-pinning.md` and
> `docs/adr/002-m3-trait-scoping.md`). The two sequences are
> independent.

### ADR-001 — Jujutsu over Git for VCS storage
**Decision:** Use Jujutsu as the storage layer.
**Rationale:** Jujutsu's operation log provides undo of any operation (not just
commits), first-class conflict resolution, and a cleaner API than libgit2.
The cost is API instability in `jj-lib`; mitigated by pinning and tracking upstream.
**Consequence:** Do not expose Git-specific concepts (branches as refs, staging area)
in the `ContentStore` interface. The interface must be VCS-agnostic.
**See also:** `docs/adr/001-jujutsu-pinning.md` (filesystem stand-in for
Phase 1; `jj-lib` integration deferred to TASK-004).

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
**Mechanism:** The
`require_permission!(index, principal, action, resource)` macro from
`liquid-permissions` is the canonical first line of every bridge / CLI
entrypoint; it awaits `PermissionIndex::check` and short-circuits with
`Err(LiquidError::Forbidden)` on denial.
**See also:** `docs/adr/002-m3-trait-scoping.md` for the trait shape
that this enforcement point depends on.

### ADR-005 — Storage interface is abstract from the first line
**Decision:** `ContentStore`, `ReadCache`, and `PermissionIndex` are traits.
Phase 1 ships in-process / on-disk stubs; phase 3 swaps in distributed
implementations.
**Rationale:** Retrofitting abstraction across storage callsites costs more than
the initial discipline.
**Consequence:** Application code never imports `JujutsuContentStore`,
`FilesystemContentStore`, `InProcessCache`, or `InMemoryPermissionIndex`
directly. Dependency injection at startup only.
**See also:** `docs/adr/001-jujutsu-pinning.md` (the filesystem stand-in
exercises this abstraction in M2); `docs/adr/002-m3-trait-scoping.md`
(the M3 trait shapes ship the Phase-1 subset behind the same abstraction).

### ADR-006 — Components communicate only through data slots
**Decision:** Components may not hold direct Dart references to other components.
All cross-component communication goes through the `SlotBroker`.
**Rationale:** Direct references couple components, violate the cross-app
compatibility contract, and create security gaps (one component reading another's
internal state).
**Consequence:** The component runtime must enforce this — there is no legitimate
reason for a component widget to receive another component widget as a constructor
argument. See §11 "Component Isolation Enforcement" for the concrete mechanisms.

### ADR-007 — Component isolation uses widget tree scoping, not Dart isolates
**Decision:** Component isolation is enforced via `LiquidComponentScope`
`InheritedWidget` scoping + static lint rules. Dart isolates are not used.
**Rationale:** Dart isolates would achieve memory isolation but require
serialising all data crossing the isolate boundary, including every widget
rebuild payload. For a UI framework with high-frequency slot events and shared
Flutter widget trees, this overhead is unacceptable. Widget tree scoping
achieves the same logical isolation (a component cannot address a sibling's
context) at zero runtime cost. The lint rule (`no_cross_component_reference`)
catches violations statically at app-build time, not at runtime.
**Rejected alternative:** Dart isolates per component. Ruled out because:
(a) Flutter widget trees cannot span isolates — each isolate needs its own
Flutter engine instance, multiplying memory per component; (b) slot value
serialisation adds latency that would break the < 5 ms p99 emit contract.
**Consequence:** Component isolation is a Dart-level contract enforced by the
SDK and linter, not an OS-level memory boundary. Malicious Dart code in a
component could in theory bypass it — this is mitigated by signed manifests
and registry review, not by isolates.

### ADR-008 — `liquid_sdk` is the exclusive platform API for app developers
**Decision:** Apps published to the registry may only import `liquid_sdk` and
pure-Dart packages. Platform-specific Flutter plugins and `dart:io` are banned.
**Rationale:** The write-once-run-everywhere guarantee depends on `liquid_sdk`
being the complete abstraction over platform capabilities. Any platform import
in an app breaks the guarantee for at least one of the five targets and
introduces a dependency on capabilities the Liquid runtime cannot mediate
(security, permissions, telemetry).
**Consequence:** When a developer needs a platform capability not yet in
`liquid_sdk`, the correct action is to add it to the SDK. This creates a
virtuous cycle: each new capability request improves the SDK for all developers.
Registry CI enforces the ban; a package failing `flutter build` on any target
is rejected. The SDK ships a custom `liquid_sdk_lint` package that makes
violations analyzer errors during development.

---

## 16. Release Process

Releases are driven by Conventional Commit history using **`cargo-release`**
(Rust crates) and matching version tags for the Flutter packages.

### Setup (one-time)

```sh
cargo install cargo-release
```

### Version strategy

Liquid follows **semver**. Version numbers are managed per crate/package.
The `liquid-core` crate version is the canonical version for the overall
release; SDK and CLI versions match it.

### Release workflow

```sh
# 1. Ensure main is clean and all CI passes
just check

# 2. Dry run to preview what will change
cargo release --manifest-path core/Cargo.toml patch --dry-run
# (use: patch | minor | major)

# 3. Execute the release — bumps versions, commits, tags, pushes
cargo release --manifest-path core/Cargo.toml patch

# 4. Publish Dart SDK to pub.dev (or self-hosted registry)
cd sdk/liquid_sdk && flutter pub publish

# 5. Publish apps to the Liquid registry
just cli -- registry publish apps/text_editor/
just cli -- registry publish apps/spreadsheet/
just cli -- registry publish apps/chart/
```

### `cargo-release` configuration (`core/release.toml`)

Create this file when first setting up releases:

```toml
# core/release.toml
sign-commit = false          # enable when GPG is configured
push = true
publish = false              # Rust crates are internal; not published to crates.io
tag-name = "v{{version}}"
pre-release-commit-message = "chore(release): prepare v{{version}}"
```

### Changelog

Conventional Commits history generates the changelog automatically.
Keep commit messages clean — `feat` and `fix` types appear in release notes;
`chore`, `refactor`, `test`, `docs` are grouped under "Other Changes" or omitted.

### Release artefacts

| Artefact | Where |
|---|---|
| `liquid` CLI binary | GitHub Release assets (built by CI on tag push) |
| Flutter app (desktop) | GitHub Release assets |
| `liquid_sdk` Dart package | pub.dev or self-hosted registry |
| Reference apps | Liquid package registry |
