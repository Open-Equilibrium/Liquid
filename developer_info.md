# Liquid — Developer Information

> Architecture, design rationale, and project context for contributors and
> people evaluating Liquid in depth. The user-facing project description is
> in [`README.md`](README.md); the day-to-day contributor workflow is in
> [`CONTRIBUTING.md`](CONTRIBUTING.md); the milestone-by-milestone build
> guide is in [`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md).

---

## Table of Contents

1. [Vision](#vision)
2. [Core Design Principles](#core-design-principles)
3. [Core Concepts](#core-concepts)
4. [SDK Overview](#sdk-overview)
5. [Technology Stack](#technology-stack)
6. [Self-Hosting](#self-hosting)
7. [Feasibility Assessment](#feasibility-assessment)
8. [Risk Register](#risk-register)
9. [Competitive Landscape](#competitive-landscape)
10. [Phasing Summary](#phasing-summary)

## Vision

Liquid is a universal application platform — part UI framework, part SDK, part operating environment — that lets developers build rich, composable applications and lets users run, arrange, and govern those applications uniformly across Linux, Windows, macOS, iOS, and Android.

Where today's cross-platform stacks force trade-offs between native feel and developer ergonomics, Liquid targets both: a Rust core for performance and safety, a Flutter/Dart surface for a single consistent UI across every platform, and a component model that is as composable as Notion blocks but without Notion's walled garden.

Agents are not an afterthought. Liquid is designed from the ground up for environments where AI agents and human users operate at equal standing — sharing the same identity model, the same permission system, and the same VCS audit trail. Agents interact through a structured CLI rather than a graphical UI, making them efficient at scale without requiring any rendering infrastructure.

Performance, security, and scalability are non-negotiable constraints, not post-launch concerns. Every architectural decision in Liquid is evaluated against the target of 10 000+ human users, 10 000+ agents, millions of files, and millisecond operation latency.


## Core Design Principles

These are not aspirational goals — they are hard constraints that every architectural decision must satisfy from the first commit.

### Performance

- **Target:** operations complete in milliseconds at full scale — 10 000+ concurrent users and agents, millions of files, **measured per workspace**
- **Mechanism:** the VCS layer (Jujutsu) is the durable write-ahead log and source of truth; it is never the hot read path
- A **content-addressable read cache** (Redis-class) sits in front of the VCS. VCS objects are immutable by content hash, so cache invalidation is exact and cheap — a commit evicts only the changed hashes
- All latency-sensitive reads (page loads, component data, permission checks) are served from the cache; the VCS is consulted only on cache miss or write
- The data binding event bus is backpressure-aware; slow consumers do not stall fast producers

### Scalability

**Scope: per workspace.** Liquid's scalability targets apply at the workspace level. A single Liquid installation hosts many independent workspaces. Each workspace scales on its own to 10 000+ human users, 10 000+ agents, and millions of files. One workspace's load never affects another. The total capacity of a Liquid installation is the sum of its workspaces and scales horizontally by adding nodes, not by redesigning the application.

- **Target:** horizontal scaling with no single-node bottleneck at any layer
- The Liquid server is stateless between requests; session state is stored in the cache layer
- The permission system uses a **materialized permission index** — RBAC evaluation is a single key lookup, not a live graph traversal; the index is updated asynchronously on role/policy changes
- VCS write throughput scales via **partitioning by workspace**; each workspace is an independent Jujutsu repository — one workspace's commit load has zero impact on another
- For multi-region deployments: a change event bus (Kafka-class) fans out commits to replica nodes; reads are local, writes are primary-with-async-replication
- Agent workloads (which can be highly parallel) use the same stateless request path as human users — no special agent infrastructure needed

### Security

- **Zero-trust between components:** components are sandboxed; one component cannot access another component's data without an explicit data binding wired by a user with sufficient permission
- **Capability-based permissions:** apps and agents receive only the minimum capabilities declared in their manifest; no implicit ambient authority
- **Signed manifests:** every app, component, and extension package is cryptographically signed; the runtime refuses to load unsigned or tampered packages
- **Agent authority limit:** an agent can never hold permissions exceeding those of the human principal who authorized it
- **Audit by default:** all reads and writes — human or agent — are logged in the VCS operation log; the log is append-only and cannot be modified without creating a detectable fork
- **Security reviews are a gate**, not a recommendation, before any public registry opens

### Stability

- **Typed contracts everywhere:** data binding slots are typed and versioned; a component cannot silently break a consumer by changing its output shape — schema changes require a version bump and a migration path
- **No implicit side effects:** every state mutation goes through the VCS commit path; there is no out-of-band write mechanism
- **Graceful degradation:** if a component fails, the rest of the page continues to render; component isolation prevents cascading failures
- **API stability windows:** SDK APIs follow semantic versioning with a documented deprecation period; apps built against v1 of the SDK continue to work until v1 is formally retired


## Core Concepts

### Abstraction Model

Every entity in Liquid sits at a well-defined layer. Understanding the hierarchy is the fastest way to understand the whole system.

```
User
└── Workspace  (1..*)            personal, business, client project, …
    ├── Page  (0..*)             detail view; navigable in explorer like Notion
    │   └── Grid                 place and size any app instance or component
    └── App Instance  (1..*)     each installation of an app in this workspace
        ├── Tenant               the data / config context for this instance
        ├── Component  (1..*)    atomic unit of UI + logic; placeable on any page
        └── Extension  (0..*)    only if the app declares extension points
```

**User** — a single identity across the whole Liquid installation. One login, multiple workspaces.

**Workspace** — the top-level isolation boundary. Each workspace has its own pages, app instances, users, agents, permissions, and VCS repository. Switching workspace switches the entire context.

**Page** — the detail view on the right side of the shell. Pages are organised in a tree in the explorer (with icons, subpages, drag-and-drop — the same as Notion). Opening a page shows its grid. A user places any combination of app instances and individual components onto the grid, sizes them across cells, and arranges them freely. The page layout is itself versioned.

**App Instance** — an app assigned to a workspace. The same app type can be installed multiple times; each installation is a separate instance with its own tenant.

**Tenant** — the data and configuration context for one app instance. Two instances of the same app in the same workspace can connect to entirely different data sources via different tenants. Tenant is an app-instance-level concept — it is not managed at the workspace level.

**Component** — the atomic building block inside an app. Components are the units that appear in grid cells, bind data between each other, and can be reused across app instances.

**Extension** — enriches an existing app instance without forking it. Only available when the app itself declares that it supports extension points.

---

### Explorer

The left panel of the Liquid shell. It is always scoped to the active workspace — everything visible belongs to or is accessible within that workspace.

**Workspace switcher**

A compact picker at the top of the explorer lets the user switch between their workspaces (e.g., Personal, Business, Client A). Switching workspace reloads the entire explorer and page context. The user's identity stays the same; only the active scope changes.

**Page tree**

Pages are first-class entities in the explorer, organised in a hierarchy:

- Pages can be nested as subpages to arbitrary depth, similar to Notion
- Each page can carry a custom icon (emoji, image, or app-defined icon)
- Drag-and-drop reordering within the tree
- Inline renaming
- Right-click context menu for creating subpages, moving, duplicating, or deleting

**App instances**

Each app instance assigned to the workspace appears in the explorer alongside pages. If the same app is installed twice with different tenants (e.g., two instances of a spreadsheet app pointing to different data sources), both instances are listed by their user-facing name, keeping them visually distinct.

**Tags and custom sections**

Beyond the page tree, the explorer supports a structured tag and filter system:

- **Tags** — arbitrary labels attached to any entity (app instance, component, page, document)
- **Custom sections** — user-defined groupings backed by pattern-matching filter rules
- **Visibility rules** — hide or surface content based on tag combinations, ownership, or role within the workspace

The explorer is fully user-configurable. Power users can replicate a VS Code–style file tree, a Notion-style page hierarchy, or a flat Obsidian-style tag cloud using the same underlying mechanism.

---

### Pages and the Grid

A **page** is the detail view — the main content area on the right side of the shell. Pages are the primary canvas where users compose their workspace. They are navigated and organised in the explorer exactly like Notion: nested in a tree, each with a custom icon, dragable into any order, with subpages to any depth.

**What goes on a page**

A page is a free composition surface. The user can place on it:

- Any **app instance** installed in the current workspace (identified by its name and tenant)
- Any individual **component** from any app instance, without needing to place the whole app
- Any mix of the above, in any combination

Everything placed on a page is sized and arranged using the grid.

**Grid behaviour**

The grid is a fixed coordinate system of columns and rows — it does not resize dynamically. Items placed on it snap to grid boundaries and can span freely:

- An item can occupy a single cell, or span multiple columns and rows
- Any item can be maximised to fill the entire page
- Items are repositioned by dragging to a new grid location
- Resize handles let the user expand or contract an item across more or fewer cells

This gives the predictability of a structured layout with the flexibility of a widget canvas — similar to a mobile home screen where each app occupies a discrete but resizable slot.

**Pages in the explorer**

Pages appear in the explorer's page tree. Each page can:

- Have a custom icon and display name
- Contain subpages (nested to any depth)
- Be tagged for filtering and custom sections
- Be shared with specific users, roles, or agents within the workspace

Pages are versioned: every change to a page's layout, content, or subpage structure is a VCS commit.

---

### Workspaces

A workspace is the top-level isolation boundary in Liquid — the same concept as a workspace in Notion or Slack. Every user starts with one workspace and can create or join additional ones.

Each workspace is fully self-contained:

- Its own set of app instances, pages, and data
- Its own users and agents, each with workspace-scoped roles and permissions
- Its own VCS repository — commits in one workspace never touch another
- Its own cache partition and permission index, ensuring performance isolation at scale

A user can belong to multiple workspaces simultaneously (e.g., Personal and Business). Switching workspace in the explorer changes the entire context: pages, app instances, and effective permissions all reload. The user's identity across workspaces is the same single Liquid account.

Workspaces are the unit of scale: each workspace independently targets the 10 000+ user / 10 000+ agent / millions of files performance targets. One workspace's load has no impact on another.

---

### Apps and App Instances

**Apps** are software packages — published to Liquid's open registry or a self-hosted registry, developed against the Liquid SDK. An app defines what components it contains, what permissions it requires, what extension points it exposes, and what CLI commands it surfaces for agents. Apps are the distributable, versioned artefact.

**App instances** are what users actually work with. When a user adds an app to a workspace, they create an app instance — a specific installation of that app within that workspace. Each app instance:

- Gets a user-facing name (shown in the explorer and grid)
- Is bound to a **tenant** — the data and configuration context that tells this instance what backend, credentials, or data store to connect to
- Has its own permission scope within the workspace
- Is independently versioned in the workspace VCS

**The same app can be installed multiple times in the same workspace, each with a different tenant.** This is intentional and a first-class feature:

> *Example:* A team adds a CRM app to their Business workspace twice — one instance with a tenant pointing to the US market data store, another with a tenant pointing to the EU market data store. Both instances appear in the explorer by name ("CRM — US" and "CRM — EU"), can be placed on pages independently, and expose separate data slots. An agent can interact with each instance via its own CLI address.

App instances are the rendering units in the grid. They are not containers for components — they are the organisational and distribution unit. Components are what get rendered inside grid cells.

---

### Components and Data Binding

Components are the atomic building block of Liquid. Each component is a self-contained unit of UI and logic that can be placed independently on any page, regardless of which app originally shipped it.

**Cross-app compatibility**

A `TextEditor` component developed for App A can be embedded in App B without modification. Components declare what they need (inputs) and what they produce (outputs) in their manifest; the Liquid runtime handles the rest.

**Component data binding**

Components placed on the same page can be connected through typed data streams:

- A component declares one or more **output slots** — named, typed values it publishes whenever its state changes
- A component declares one or more **input slots** — named, typed values it consumes to update its rendering or behavior
- Users (or agents) can wire an output slot of one component to an input slot of another, creating a live data binding

**Example:** A spreadsheet component and a chart component are placed side-by-side on the same page. The spreadsheet publishes its selected data range as an output slot (`table:selectedRows`). The chart component subscribes to that slot as its data source. When the user updates a cell in the spreadsheet, the chart re-renders automatically — no copy-paste, no export, no custom integration code required.

This model replaces the idea of same-plane rendering with something more powerful: **components remain visually independent but are semantically connected**. Any number of components on a page can form a data flow graph, enabling rich workflows without coupling the components' internal implementations.

Data bindings are stored as part of the page definition and are therefore versioned and auditable like everything else in Liquid.

---

### Extensions

Extensions enrich an existing app instance without forking the app. **An extension can only be applied to an app instance if the app itself declares that it supports extension points** — apps opt in explicitly in their manifest.

When an app supports extensions:

- Extensions hook into the lifecycle events or data slots the app has declared as extensible
- They can add UI surface within the app's grid area, transform data flowing through a slot, or republish enriched values to downstream components
- They are distributed and versioned independently from the host app — an extension update does not require an app update
- Multiple extensions can be active on the same app instance simultaneously, applied in a declared order

Extensions follow the same permission model as apps: they are signed packages, require explicit user approval to install, and are subject to the same VCS audit trail. An extension cannot access any data or lifecycle event the host app has not explicitly exposed.

---

### VCS (Jujutsu-native)

All content in Liquid — pages, components, documents, bindings, configuration — is stored in a Jujutsu repository. This is not an add-on; it is the storage layer.

- Every edit is a commit: no accidental data loss
- Agent edits are attributed to the agent's identity and are reversible
- Human and agent edits appear in the same operation log, making collaborative workflows fully traceable
- Branching and merging are available to end users through the Liquid UI, not just the CLI
- Self-hosted sync via any Jujutsu-compatible remote (NAS, server, cloud)

Jujutsu was chosen over Git for its cleaner operation model, better conflict handling, and first-class support for the operation log (undo of any operation, not just file changes). For enterprise scale, Jujutsu's design also handles large monorepos better than Git.

---

### User and Permission Management

Liquid provides a unified identity and permission layer across all apps:

- Users are defined once per Liquid installation or federated via OIDC
- Permissions are scoped to: **workspace → app instance (tenant) → component → page → field**
- A user's role in one workspace is entirely independent of their role in another
- Audit log of all access is stored in the VCS operation log

---

### Agents as First-Class Citizens

Agents are not plugins, integrations, or privileged daemons in Liquid. They are principals — equal in standing to human users — with one key difference: **agents do not have a graphical UI**. Instead, every app developed for Liquid exposes an agent-native CLI surface, and agents interact with Liquid exclusively through that interface.

**Why CLI, not UI?**

A human user opens a page, sees the grid, and drags components around. An agent does not need any of that. What an agent needs is a structured, scriptable interface to read data, perform operations, and write results — with the same permission boundaries a human would face. A CLI delivers exactly that: low overhead, easy to script, easy to test, and trivially parallelizable across thousands of concurrent agents.

**Identity**

Every agent has a unique identity registered within a workspace. It authenticates the same way a human user does (token-based, OIDC-compatible) and is subject to the same session and rate-limit management. Agents are provisioned by a human administrator who holds at least the permissions being granted.

**The Liquid Agent CLI**

Each app developed against the Liquid SDK automatically exposes a CLI surface alongside its graphical interface. The CLI is not a separate integration effort — it is generated from the same app manifest and component definitions that drive the UI.

Example interactions:

```sh
# Read a page in a workspace
liquid read page/project-alpha --workspace acme-corp --as agent:research-bot

# Write a row to a component inside a specific app instance (identified by its tenant config)
liquid write app/crm-us/component/pipeline-sheet \
  --row '{"month":"May","revenue":42000}' \
  --workspace acme-corp --as agent:finance-bot

# Subscribe to a data slot on an app instance and stream updates
liquid subscribe app/crm-eu/slot/deals:updated \
  --workspace acme-corp --as agent:crm-sync
```

All commands go through the same permission checks as the equivalent UI action. An agent issuing a `write` it does not have permission for receives the same error a human would.

**Permissions**

Agents receive roles and permission scopes through the same RBAC system as humans:

- An agent can be granted read access to specific pages, components, or data fields and nothing else
- An agent cannot exceed the permissions of the human principal who provisioned it
- Permission changes take effect immediately and are reflected in the VCS audit log
- The materialized permission index (see Core Design Principles) means permission checks add sub-millisecond overhead even at 10 000+ concurrent agents within a workspace

**Agent-to-agent collaboration**

Multiple agents can be assigned to the same workspace. They collaborate through the same data binding system that components use: one agent writes to a slot on an app instance, another subscribes to it via the CLI. No special inter-agent protocol is required.

**Audit and reversibility**

Because all writes go through the VCS commit path, every action an agent takes is:

- Attributed to that agent's identity
- Timestamped
- Reversible with a single undo operation in the Liquid UI or CLI
- Visible in the operation log alongside human edits, indistinguishable in format

This makes agent work safe to allow in production environments — every change is traceable and every mistake is undoable.

---

## SDK Overview

The Liquid SDK (`sdk/liquid_sdk/`, lands in Phase 2) provides:

The Liquid SDK provides:

- **App manifest** — declare the app's identity, required permissions, supported tenant configuration schema, grid size constraints, CLI command surface, and whether the app supports extensions
- **Component protocol** — register components, declare typed input/output data slots, expose extension hooks (if the app opts in)
- **Grid API** — request layout changes, respond to resize/maximise events from the page
- **Data binding API** — publish to output slots, subscribe to input slots, define and version slot schemas
- **VCS API** — read/write versioned content within the app instance's scope, access history, create branches
- **Permission API** — query effective permissions for the current user or agent at workspace → app instance → component → field granularity (sub-millisecond, backed by the materialized index)
- **Agent CLI surface** — declare which app operations agents may invoke; the runtime generates CLI commands from the manifest automatically; no separate implementation required
- **Extension API** — implement extension hooks that the host app has explicitly exposed; unavailable if the host app does not declare extension points

Target languages: Dart (primary, via Flutter), with Rust bindings via FFI for performance-critical and platform-native components.

Detailed SDK documentation will live in [`docs/sdk-guide/`](docs/sdk-guide/) once Phase 2 is in flight. The authoritative interface specification is [`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md) §11.

---

## Technology Stack

| Layer | Technology | Rationale |
|---|---|---|
| Core runtime | Rust | Memory safety, performance, cross-platform compilation |
| UI shell — all platforms | Flutter (Dart) | Single codebase for Linux, Windows, macOS, iOS, Android; Impeller GPU renderer; no WebView |
| App/component logic | Dart | Consistent with Flutter; strong typing; good LLM tooling support |
| Rust ↔ Dart bridge | Flutter FFI + `flutter_rust_bridge` | Zero-copy calls between Dart UI layer and Rust core |
| VCS storage | Jujutsu | Operation log, cleaner conflict model vs. Git, large-repo performance |
| Read cache | Redis-class distributed cache | Sub-millisecond warm reads; content-addressed = exact invalidation |
| Permission index | Materialized key-value store | Single-lookup permission checks at 20 000+ concurrent principals |
| Event bus | Kafka-class message bus | Fan-out for multi-region replication and data binding at scale |
| Agent interface | Liquid Agent CLI (generated from app manifest, implemented in Rust) | Structured, scriptable, zero rendering overhead |
| Package registry | Self-hosted, open protocol | No vendor lock-in |


---

## Self-Hosting

Liquid is designed to run without any external subscription:

- The Liquid shell and registry can be deployed on a personal server or NAS
- VCS remotes are standard Jujutsu remotes (SSH, HTTP)
- User management is local or federated (OIDC-compatible)
- No telemetry by default

Operational documentation (deployment topology, backup, multi-region) will live in `docs/operations/` once Phase 3 is in flight.

---

## Feasibility Assessment

### What Is Technically Sound

**Cross-platform UI shell (Linux, Windows, macOS, iOS, Android)**
Flutter 3.x is in its Production Era (as of late 2025). The Impeller GPU renderer is fully stable on all five target platforms, delivering consistent 60/120 fps without WebView jank or shader compilation stutter. Flutter is the only single-codebase framework that covers all five of Liquid's target platforms with a non-WebView rendering pipeline. AppFlowy — a direct Notion alternative — ships a Notion-quality editor and sidebar in Flutter, proving the visual target is reachable.

**Grid layout engine**
Flutter's widget system supports arbitrary custom layouts. A static grid with cell spanning and drag-and-drop reordering is straightforward to implement in Flutter using `CustomMultiChildLayout` or a purpose-built grid widget. This is less work than the equivalent in web CSS because Flutter has no browser compatibility surface to navigate.

**Component data binding**
Typed publish/subscribe between components is a well-understood pattern (RxJS, spreadsheet cell references, Unix pipes). Implementing it as a first-class SDK primitive is architecturally novel for a UI framework but not technically risky.

**Workspace isolation and per-instance tenant configuration**
Workspace-level isolation (independent VCS repo, cache, permission index) and per-app-instance tenant configuration are standard enterprise software patterns. Well-documented, well-tested.

**VCS-backed storage with caching**
Using a VCS as a content store is unconventional but sound. Jujutsu's operation log and content-addressed object model are particularly well-suited: objects are immutable by hash, making a Redis-class read cache trivially correct. Every major collaborative platform (Figma, Notion, Linear) uses a write-ahead log as durable storage with a caching layer for hot reads — Liquid's architecture follows the same proven pattern.

**Enterprise-scale permission system**
A materialized permission index (RBAC evaluated once on policy change, results stored for O(1) lookup) is the standard approach for fine-grained permissions at tens of thousands of concurrent principals. This is operationally non-trivial but architecturally well-understood.

**Dart as SDK language**
Dart is strongly typed, compiles ahead-of-time, and has well-established conventions for widget development. LLMs (Claude, Gemini, GPT-4) handle idiomatic Dart and Flutter patterns reliably in 2026. Flutter 3.41 ships a Dart MCP server that gives AI coding tools deep project context, directly improving LLM-assisted development. The ecosystem is smaller than TypeScript but mature enough: pub.dev hosts 50 000+ packages, and Flutter has first-class support in every major IDE.

**Agent-as-principal with CLI interface**
Treating AI agents as first-class RBAC principals with a generated CLI surface is straightforward to implement once the RBAC model and app manifest format exist. The CLI generation pattern (manifest → commands) is the same approach used by tools like the AWS CLI and Kubernetes kubectl. The CLI is implemented in Rust and is independent of the Dart/Flutter layer entirely.

**Extension system**
Hook-based extension architectures are mature (VS Code's extension API is the gold standard). Replicating a restricted version of this is achievable.

### What Is Technically Novel and Hard

**Cross-app component protocol (MEDIUM-HIGH difficulty)**

For a component built by Developer A to run correctly inside an app built by Developer B, both must conform to a shared protocol for Flutter widget tree ownership, gesture and focus routing, layout constraints, data slot types, and security sandboxing. Flutter's widget model (everything is a widget, layout is constraint-based) is actually well-suited to this: components receive a `BoxConstraints` from the grid and lay themselves out within it, which is exactly how Flutter's native layout system works. The harder parts are gesture disambiguation between adjacent components and the security boundary — a component must not be able to read another component's state without an explicit data binding. Shipping 5–10 first-party apps that exercise the protocol is the only way to validate it before opening it to third parties.

**Scalable VCS + caching layer (MEDIUM difficulty)**

Achieving millisecond latency at enterprise scale requires a caching and indexing layer in front of Jujutsu (see Core Design Principles). The architecture is standard, but the interface between the application layer and the storage/cache layer must be defined cleanly in phase 1. Retrofitting it later means rewriting application code. The effort is non-trivial but the approach is well-understood.

**Jujutsu ecosystem maturity (MEDIUM difficulty)**

Jujutsu's library API (jj-lib) is not yet stable. Building a production system on top of it means accepting ongoing migration cost as Jujutsu evolves. Git would be more stable but operationally inferior for the use cases Liquid needs.

### Why Flutter as the Universal UI Layer

Liquid targets five platforms: Linux, Windows, macOS, iOS, and Android. One of its explicit differentiators is mobile-native UX — not a mobile port, not a responsive web view, but a first-class mobile experience. That requirement, combined with the single-SDK goal (one component codebase runs everywhere), narrows the choice considerably.

**Why not a web-based approach (Tauri / Electron)?**

Tauri and Electron both render UI inside a WebView or bundled Chromium. On desktop they work well. On mobile, Tauri wraps the UI in WKWebView (iOS) or the Android System WebView. This is exactly the architecture Notion used before their multi-year migration away from it — and it is the reason Notion's mobile app is still criticised for performance and UX feel in 2026. A WebView rendering pipeline on mobile hardware introduces jank, gesture approximation, and a ceiling that compounds over time as the app grows more complex. For Liquid's grid with live data-binding components, that ceiling would be hit early.

**Why Flutter specifically?**

Flutter's Impeller renderer (fully stable on all platforms as of Flutter 3.22) draws every pixel through its own GPU pipeline — no WebView, no platform widget intermediary. The result is consistent 60/120 fps on mobile and desktop alike. Critically:

- **One codebase, five targets.** The same Dart component code runs on Linux, Windows, macOS, iOS, and Android. App developers write a component once.
- **Notion/Claude-quality UX is proven.** AppFlowy — a direct open-source Notion alternative — ships a full block editor, sidebar, page tree, and drag-and-drop in Flutter. The visual and interaction quality bar is demonstrably reachable.
- **Rust core is fully preserved.** The Flutter layer calls into the Rust core via FFI using `flutter_rust_bridge`, which generates type-safe Dart bindings from Rust code. VCS, permissions, caching, and the agent CLI all live in Rust. Flutter is strictly the rendering and input layer.
- **LLM development support is solid.** Claude Code, Gemini Code Assist, Cursor, and Windsurf all support Dart/Flutter in 2026. Flutter 3.41 ships a Dart MCP server that gives AI tools deep project context. For standard Flutter patterns, LLM assistance is reliable.
- **Desktop support is production-ready.** Flutter desktop (Linux, Windows, macOS) is in production use at scale in 2026 — the Canonical Ubuntu installer ships in Flutter, and the ecosystem of desktop-ready packages has matured significantly.

**The trade-off: Dart instead of TypeScript**

Flutter requires Dart for UI code. Dart is not TypeScript: the hiring pool is smaller, the ecosystem (pub.dev, ~50 000 packages) is smaller than npm, and developers unfamiliar with Flutter face a learning curve. This is the primary cost of the Flutter decision.

It is the right trade-off for Liquid because the single-SDK requirement means the alternative is not "TypeScript and the web ecosystem" — it is "TypeScript on desktop plus Dart on mobile" (two codebases, developers implement components twice). Given that choice, committing fully to Dart and getting a single consistent stack is clearly preferable.

**The Rust ↔ Dart boundary**

All business logic, storage, permissions, and agent CLI stay in Rust. Flutter calls into Rust via `flutter_rust_bridge`, which:
- Generates type-safe async Dart bindings from annotated Rust functions
- Handles threading automatically (Rust runs on a separate thread pool; Dart receives results via `Future`/`Stream`)
- Supports zero-copy transfer of large byte buffers

This means the performance-critical path (VCS operations, cache lookups, permission checks) never touches Dart. Dart is responsible only for rendering and user input.


---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Dart/Flutter contributor pool smaller than TypeScript | Medium | Medium | Strong onboarding docs; LLM tooling (Claude Code + Dart MCP server) lowers the ramp; AppFlowy ecosystem provides reusable components |
| Flutter desktop ecosystem gaps for niche platform integrations | Low | Low | Platform channels allow thin Swift/Kotlin/C++ bridges for any missing API; this is the standard Flutter escape hatch |
| `flutter_rust_bridge` API changes break the Dart ↔ Rust boundary | Low | Medium | Pin the bridge version; the boundary is narrow and well-defined; migrations are tractable |
| Cross-app component protocol fails to attract third-party developers | Medium | High | Ship 5–10 first-party apps to prove the protocol before opening it |
| Jujutsu API instability causes rework | Medium | Medium | Pin jj-lib version; track upstream; budget migration time each release |
| Cache/permission index under-designed in phase 1, requires rewrite | Medium | High | Define clean storage and permission interfaces in phase 1; cache layer can be stubbed but the interface must be final |
| Scope creep kills momentum | High | High | Phase strictly; cut features ruthlessly in v1 |
| Open-source contributor acquisition | Medium | Medium | Strong developer docs and a compelling demo app shipped early |
| Security vulnerability in extension sandboxing | Medium | High | Capability-based permissions; mandatory security audit before public registry opens |
| Agent identity spoofing or privilege escalation | Medium | High | Agent auth is a first-class security surface; zero-trust between agent and host from the first implementation |
| Jujutsu write throughput ceiling under heavy agent workloads | Low | High | Partition by workspace from day one; each workspace is an independent Jujutsu repo; scale horizontally |


---

## Competitive Landscape

| Project | Overlap | Key difference |
|---|---|---|
| **AppFlowy** | Closest: open-source Notion alternative, Rust + Flutter | No component SDK, no cross-app data binding, no enterprise permission model, no agent layer |
| **Tauri** | Cross-platform Rust desktop shell | WebView on mobile; Liquid uses Flutter for a consistent non-WebView renderer on all five platforms |
| **Electron** | Cross-platform desktop | Desktop only, heavy (165 MB installer), no VCS, no component protocol |
| **Notion** | UX inspiration, block model, page tree | Closed, SaaS, no SDK, no VCS, no developer framework; mobile performance known weak point |
| **Obsidian** | Explorer UX, extensibility | Note-taking only, no cross-app component protocol, Git plugin not native |
| **ClickUp** | Card/widget layout inspiration | SaaS, closed ecosystem, no component protocol |
| **React Native** | Cross-platform mobile + some desktop | Notion abandoned it; JS bridge bottleneck; no VCS, no component protocol |

Liquid's combination of **open SDK + cross-app data-binding components + VCS-native + agent-as-principal + enterprise scale** has no direct equivalent. The risk is not that competitors exist but that the scope is so large it is hard to reach a compelling v1.

---

## Phasing Summary

The authoritative milestone-by-milestone breakdown lives in
[`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md). The summary below
mirrors its four-phase structure; the table at the top of [`README.md`](README.md#status)
tracks current progress against it.

**Phase 1 — Rust core + Flutter shell skeleton (12–18 months, small team)**

- Rust workspace bootstrap: `liquid-core` primitives (`WorkspaceId`, `PrincipalId`, `ContentHash`, …) and `LiquidError`
- `liquid-vcs`: Jujutsu-backed `ContentStore` — one repo per workspace from day one (workspace partitioning is non-negotiable from the first line of storage code)
- `liquid-auth`: file-backed local users + agents (Argon2id + HMAC tokens); OIDC deferred to phase 3
- `liquid-permissions`: `InMemoryPermissionIndex` stub behind the `PermissionIndex` trait; built-in roles hard-coded
- `liquid-cache`: `InProcessCache` stub behind the `ReadCache` trait
- `liquid-sdk-bridge` FFI: `create_workspace`, `list_workspaces`, `load_page`, `write_page`, `check_permission` — every call gates on `require_permission!`
- Flutter desktop shell (Linux, Windows, macOS): `RootShell`, `WorkspaceSwitcher`, `ExplorerPanel` (page tree + app instance list + tag sections), `PageGrid` with drag/resize/maximise on a placeholder `GridItem`
- Rust agent CLI subset: `workspace create|list`, `page read|write`, `auth provision-agent|token`

Success criterion: desktop app launches on all three desktop targets; user can create a workspace, open a page, and drag a placeholder grid item; an agent can be provisioned and perform a versioned page write via CLI.

**Phase 2 — SDK + first-party apps (6–9 months)**

- Public Dart SDK (`liquid_sdk`): `AppManifest`, `ComponentManifest`, `LiquidComponent`, `SlotSchema`, `GridApi`, `VcsApi`, `PermissionApi`
- `liquid-bindings`: `InProcessSlotBroker` + slot wiring UI (long-press output → drag to input); wirings persisted in the workspace VCS at `.liquid/pages/<page_id>/bindings.json`
- Multi-instance tenant configuration: per-instance encrypted tenant config (AES-256-GCM, key from Argon2id over workspace-owner password); JSON-Schema-driven install form
- First-party reference apps in Dart: TextEditor, Spreadsheet, Chart — exercise the cross-app component protocol and prove the data binding contract
- Signed manifests enforced by default in release builds (Ed25519); registry CI pipeline wired
- Agent CLI extends to app-instance addressing: `liquid app <instance-name> read|write|slot subscribe|slot publish`

Success criterion: a developer builds a Liquid app with data-binding components in Dart in under a day, installs it twice in the same workspace with different tenant configs, and an agent interacts with each instance independently via CLI.

**Phase 3 — Mobile + scale + extensions (6–12 months)**

- Mobile targets (iOS, Android) — same Flutter/Dart codebase from phases 1–2; gesture audit for touch + 44 pt minimums; explorer collapses to a bottom sheet on narrow screens
- `RedisCache` swapped in behind the `ReadCache` trait (feature flag `distributed-cache`); `MaterializedPermissionIndex` swapped in behind `PermissionIndex` — zero application-code changes
- OIDC identity provider integration (Google, Microsoft, generic OpenID Connect)
- Extension API: apps declare `ExtensionPoint`s in their manifest; signed extensions hook lifecycle events / slot transforms in the host app's restricted context
- Self-hosted registry: REST `POST /packages` with signature verification; `liquid registry publish|install`; per-workspace trusted signing keys

**Phase 4 — Ecosystem + high availability**

- Kafka-class event bus replacing in-process slot broadcast (feature flag `distributed-bus`); per-workspace topics; lag-based backpressure
- Multi-region Jujutsu replication: per-workspace primary, async replication via the event bus, RPO ≤ 1 commit
- Scale hardening: k6 load tests at 10 000 concurrent users per workspace sustained 30 minutes; profile against the SDK Performance Contract bounds
- Community app ecosystem opens once the security review and scale targets are signed off

---

## Closing notes

Liquid addresses real, under-served problems: VCS-native content, cross-app data-binding composability, agents as genuine first-class principals operating through a structured CLI, and a platform built for enterprise scale without SaaS lock-in.

Performance, security, scalability, and stability are not aspirational — they are design constraints enforced from phase 1. The key architectural decisions that make the scale targets achievable are all standard, proven patterns: content-addressed caching, a materialized permission index, workspace-partitioned storage, and a stateless request path. None of them are exotic; all of them must be designed for early — phase 1 ships them as in-process stubs behind stable trait interfaces, so phase 3 can swap in the distributed implementations without touching application code.

Flutter as the universal UI layer resolves the mobile question cleanly: one Dart codebase targets all five platforms through a consistent GPU-rendered pipeline. There is no WebView ceiling, no platform-specific rendering divergence, and no split SDK. Mobile arrives in phase 3 as a build target, not a separate workstream. The Rust ↔ Dart boundary via `flutter_rust_bridge` keeps all business logic, storage, and security in Rust, where it belongs.

The primary execution risk remains **scope**. The path to success is shipping a desktop v1 in Flutter that demonstrates the SDK, data binding, agent CLI, and VCS audit trail working together convincingly. That is the proof of concept that attracts contributors and validates the protocol before it is opened to the community.

With disciplined phasing, Liquid is feasible. Without it, it joins the long list of ambitious open-source platforms that never shipped.
