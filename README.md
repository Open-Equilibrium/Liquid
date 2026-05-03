# Liquid

> An open-source, cross-platform UI framework and SDK for building apps, components, and extensions — with VCS, user management, and agent-ready permissions built in from day one. Designed for enterprise scale: 10 000+ users, 10 000+ agents, millions of files, millisecond latency.

---

## Table of Contents

1. [Vision](#vision)
2. [Why Liquid?](#why-liquid)
3. [Core Design Principles](#core-design-principles)
4. [Core Concepts](#core-concepts)
   - [Explorer](#explorer)
   - [Pages and the Grid](#pages-and-the-grid)
   - [Apps](#apps)
   - [Components and Data Binding](#components-and-data-binding)
   - [Extensions](#extensions)
   - [Tenants](#tenants)
   - [VCS (Jujutsu-native)](#vcs-jujutsu-native)
   - [User and Permission Management](#user-and-permission-management)
   - [Agents as First-Class Citizens](#agents-as-first-class-citizens)
5. [SDK](#sdk)
6. [Technology Stack](#technology-stack)
7. [Self-Hosting](#self-hosting)
8. [Feasibility Assessment](#feasibility-assessment)
   - [What Is Technically Sound](#what-is-technically-sound)
   - [What Is Technically Novel and Hard](#what-is-technically-novel-and-hard)
   - [Why Flutter as the Universal UI Layer](#why-flutter-as-the-universal-ui-layer)
   - [Risk Register](#risk-register)
   - [Competitive Landscape](#competitive-landscape)
   - [Recommended Phasing](#recommended-phasing)
9. [Conclusion](#conclusion)

---

## Vision

Liquid is a universal application platform — part UI framework, part SDK, part operating environment — that lets developers build rich, composable applications and lets users run, arrange, and govern those applications uniformly across Linux, Windows, macOS, iOS, and Android.

Where today's cross-platform stacks force trade-offs between native feel and developer ergonomics, Liquid targets both: a Rust core for performance and safety, a Flutter/Dart surface for a single consistent UI across every platform, and a component model that is as composable as Notion blocks but without Notion's walled garden.

Agents are not an afterthought. Liquid is designed from the ground up for environments where AI agents and human users operate at equal standing — sharing the same identity model, the same permission system, and the same VCS audit trail. Agents interact through a structured CLI rather than a graphical UI, making them efficient at scale without requiring any rendering infrastructure.

Performance, security, and scalability are non-negotiable constraints, not post-launch concerns. Every architectural decision in Liquid is evaluated against the target of 10 000+ human users, 10 000+ agents, millions of files, and millisecond operation latency.

---

## Why Liquid?

| Problem today | Liquid's answer |
|---|---|
| Every app reinvents user management, roles, and permissions | First-class user/permission layer shared across all apps |
| Data loss from agents editing production content | VCS native — every change is versioned, reversible, and attributable |
| Mobile apps are second-class citizens in "cross-platform" tools | Mobile-first grid layout and rendering pipeline |
| Subscriptions required for core functionality | Fully self-hostable; no SaaS dependency |
| Components are siloed inside apps, data cannot flow between them | Cross-app component data binding: any component can publish and consume typed data streams |
| Agent access is coarse-grained or uncontrolled | Agents are first-class principals with identity, roles, and fine-grained permissions; they operate via a structured CLI, not a GUI |
| Frameworks are not built for enterprise scale | Scalability, performance, security, and stability are core design targets from day one — not retrofits |

---

## Core Design Principles

These are not aspirational goals — they are hard constraints that every architectural decision must satisfy from the first commit.

### Performance

- **Target:** operations complete in milliseconds at full scale (10 000+ concurrent users and agents, millions of files)
- **Mechanism:** the VCS layer (Jujutsu) is the durable write-ahead log and source of truth; it is never the hot read path
- A **content-addressable read cache** (Redis-class) sits in front of the VCS. VCS objects are immutable by content hash, so cache invalidation is exact and cheap — a commit evicts only the changed hashes
- All latency-sensitive reads (page loads, component data, permission checks) are served from the cache; the VCS is consulted only on cache miss or write
- The data binding event bus is backpressure-aware; slow consumers do not stall fast producers

### Scalability

- **Target:** horizontal scaling with no single-node bottleneck at any layer
- The Liquid server is stateless between requests; session state is stored in the cache layer
- The permission system uses a **materialized permission index** — RBAC evaluation is a single key lookup, not a live graph traversal; the index is updated asynchronously on role/policy changes
- VCS write throughput scales via partitioning by tenant; tenants are independent repositories
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

---

## Core Concepts

### Explorer

The left panel of the Liquid shell. It displays all available apps, components, pages, and tagged content. The explorer is the primary navigation surface for everything a user owns or has access to.

**Page tree**

Pages are first-class entities in the explorer, organized in a hierarchy:

- Pages can be nested as subpages to arbitrary depth, similar to Notion
- Each page can carry a custom icon (emoji, image, or app-defined icon)
- Drag-and-drop reordering within the tree
- Inline renaming
- Right-click context menu for creating subpages, moving, duplicating, or deleting

**Tags and custom sections**

Beyond the page tree, the explorer supports a structured tag and filter system:

- **Tags** — arbitrary labels attached to any entity (app, component, page, document)
- **Custom sections** — user-defined groupings backed by pattern-matching filter rules
- **Visibility rules** — hide or surface content based on tag combinations, ownership, or tenant scope

The explorer is fully user-configurable. Power users can replicate a VS Code–style file tree, a Notion-style page hierarchy, or a flat Obsidian-style tag cloud using the same underlying mechanism.

---

### Pages and the Grid

The right-hand content area is called a **page**. A page is organized by a fixed grid — a coordinate system of rows and columns that provides stable anchor points for layout.

**Grid behavior**

- The grid itself is static: its columns and rows do not move or resize dynamically
- Apps and components placed on the grid are not confined to a single cell — they can span multiple columns, multiple rows, or both
- Any app or component can be maximized to fill the entire page, temporarily covering other content
- Rearranging apps means dragging them to a new grid position; they snap to grid boundaries
- Resize handles allow expanding or contracting an app across more or fewer cells

This model gives users the predictability of a fixed layout with the flexibility of a widget-based workspace — similar to a mobile home screen where apps occupy discrete slots but can vary in size.

**Pages in the explorer**

Pages appear in the explorer's page tree. Each page can:

- Have a custom icon and display name
- Contain subpages (nested to any depth)
- Be tagged for filtering and custom sections
- Be shared with specific users, roles, or agents within the tenant

Pages are versioned: every change to a page's layout, content, or subpage structure is a VCS commit.

---

### Apps

Apps are the primary unit of functionality in Liquid. They are analogous to mobile apps or ClickUp cards:

- Developed against the Liquid SDK
- Distributed through Liquid's open registry or self-hosted registries
- Rendered inside one or more grid cells on a page
- Subject to the permission model of the tenant they run in

Apps are composites of components and have no special rendering privilege over components — they are organizational and distribution units, not rendering containers.

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

Extensions enrich existing apps and components without forking them:

- Hook into lifecycle events of any app or component they have permission to extend
- Can add UI surface, transform data, or intercept and republish data slot values
- Distributed and versioned independently from the host app

Extensions follow the same permission model as apps and are subject to the same VCS audit trail.

---

### Tenants

A single Liquid user can have multiple **tenants** — isolated environments with separate app configurations, data, and permission scopes:

- Example: one tenant for personal use, one for a work organization
- Switching tenants switches the full context: the explorer content, pages, and the user's effective permissions
- Agents are assigned to tenants and inherit only that tenant's permission scope
- In an enterprise deployment, tenants map to organizational units (teams, departments, external partners)

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
- Permissions are scoped to: tenant → app → component → page → field
- Audit log of all access is stored in the VCS operation log

---

### Agents as First-Class Citizens

Agents are not plugins, integrations, or privileged daemons in Liquid. They are principals — equal in standing to human users — with one key difference: **agents do not have a graphical UI**. Instead, every app developed for Liquid exposes an agent-native CLI surface, and agents interact with Liquid exclusively through that interface.

**Why CLI, not UI?**

A human user opens a page, sees the grid, and drags components around. An agent does not need any of that. What an agent needs is a structured, scriptable interface to read data, perform operations, and write results — with the same permission boundaries a human would face. A CLI delivers exactly that: low overhead, easy to script, easy to test, and trivially parallelizable across thousands of concurrent agents.

**Identity**

Every agent has a unique identity registered within a tenant. It authenticates the same way a human user does (token-based, OIDC-compatible) and is subject to the same session and rate-limit management. Agents are provisioned by a human administrator who holds at least the permissions being granted.

**The Liquid Agent CLI**

Each app developed against the Liquid SDK automatically exposes a CLI surface alongside its graphical interface. The CLI is not a separate integration effort — it is generated from the same app manifest and component definitions that drive the UI.

Example interactions:

```sh
# Read the contents of a page the agent has access to
liquid read page/project-alpha --tenant acme-corp --as agent:research-bot

# Write a new entry to a spreadsheet component
liquid write component/budget-sheet --row '{"month":"May","cost":4200}' --as agent:finance-bot

# Subscribe to a data slot and stream updates
liquid subscribe slot/sales-pipeline:updated --as agent:crm-sync
```

All commands go through the same permission checks as the equivalent UI action. An agent issuing a `write` it does not have permission for receives the same error a human would.

**Permissions**

Agents receive roles and permission scopes through the same RBAC system as humans:

- An agent can be granted read access to specific pages, components, or data fields and nothing else
- An agent cannot exceed the permissions of the human principal who provisioned it
- Permission changes take effect immediately and are reflected in the VCS audit log
- The materialized permission index (see Core Design Principles) means permission checks add sub-millisecond overhead even at 10 000+ concurrent agents

**Agent-to-agent collaboration**

Multiple agents can be assigned to the same tenant. They collaborate through the same data binding system that components use: one agent writes to a slot, another subscribes to it via the CLI. No special inter-agent protocol is required.

**Audit and reversibility**

Because all writes go through the VCS commit path, every action an agent takes is:

- Attributed to that agent's identity
- Timestamped
- Reversible with a single undo operation in the Liquid UI or CLI
- Visible in the operation log alongside human edits, indistinguishable in format

This makes agent work safe to allow in production environments — every change is traceable and every mistake is undoable.

---

## SDK

The Liquid SDK provides:

- **App manifest** — declare dependencies, required permissions, grid size constraints, and CLI command surface
- **Component protocol** — register components, declare input/output data slots, expose extension hooks
- **Grid API** — request layout changes, respond to resize/maximize events
- **Data binding API** — publish to output slots, subscribe to input slots, define typed slot schemas
- **VCS API** — read/write versioned content, access history, create branches
- **Permission API** — query effective permissions for the current user or agent (backed by the materialized index; sub-millisecond)
- **Agent CLI surface** — declare which app operations are accessible via the Liquid agent CLI; the runtime generates the CLI commands from the manifest automatically; no separate integration effort
- **Extension API** — hook into other apps/components if permitted

Target languages: Dart (primary, via Flutter), with Rust bindings via FFI for performance-critical and platform-native components.

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

---

## Feasibility Assessment

### What Is Technically Sound

**Cross-platform UI shell (Linux, Windows, macOS, iOS, Android)**
Flutter 3.x is in its Production Era (as of late 2025). The Impeller GPU renderer is fully stable on all five target platforms, delivering consistent 60/120 fps without WebView jank or shader compilation stutter. Flutter is the only single-codebase framework that covers all five of Liquid's target platforms with a non-WebView rendering pipeline. AppFlowy — a direct Notion alternative — ships a Notion-quality editor and sidebar in Flutter, proving the visual target is reachable.

**Grid layout engine**
Flutter's widget system supports arbitrary custom layouts. A static grid with cell spanning and drag-and-drop reordering is straightforward to implement in Flutter using `CustomMultiChildLayout` or a purpose-built grid widget. This is less work than the equivalent in web CSS because Flutter has no browser compatibility surface to navigate.

**Component data binding**
Typed publish/subscribe between components is a well-understood pattern (RxJS, spreadsheet cell references, Unix pipes). Implementing it as a first-class SDK primitive is architecturally novel for a UI framework but not technically risky.

**Multi-tenant user management**
Tenant isolation, RBAC, and per-tenant app configuration are standard enterprise software patterns. Well-documented, well-tested.

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

---

### What Is Technically Novel and Hard

**Cross-app component protocol (MEDIUM-HIGH difficulty)**

For a component built by Developer A to run correctly inside an app built by Developer B, both must conform to a shared protocol for Flutter widget tree ownership, gesture and focus routing, layout constraints, data slot types, and security sandboxing. Flutter's widget model (everything is a widget, layout is constraint-based) is actually well-suited to this: components receive a `BoxConstraints` from the grid and lay themselves out within it, which is exactly how Flutter's native layout system works. The harder parts are gesture disambiguation between adjacent components and the security boundary — a component must not be able to read another component's state without an explicit data binding. Shipping 5–10 first-party apps that exercise the protocol is the only way to validate it before opening it to third parties.

**Scalable VCS + caching layer (MEDIUM difficulty)**

Achieving millisecond latency at enterprise scale requires a caching and indexing layer in front of Jujutsu (see Core Design Principles). The architecture is standard, but the interface between the application layer and the storage/cache layer must be defined cleanly in phase 1. Retrofitting it later means rewriting application code. The effort is non-trivial but the approach is well-understood.

**Jujutsu ecosystem maturity (MEDIUM difficulty)**

Jujutsu's library API (jj-lib) is not yet stable. Building a production system on top of it means accepting ongoing migration cost as Jujutsu evolves. Git would be more stable but operationally inferior for the use cases Liquid needs.

---

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

### Risk Register

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
| Jujutsu write throughput ceiling under heavy agent workloads | Low | High | Partition by tenant from day one; each tenant is an independent Jujutsu repo; scale horizontally |

---

### Competitive Landscape

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

### Recommended Phasing

**Phase 1 — Desktop shell + SDK foundation (12–18 months, small team)**

- Liquid shell for Linux, Windows, macOS — Flutter desktop (Dart + Rust core via `flutter_rust_bridge`)
- Explorer panel with page tree (icons, subpages, drag-and-drop) and tag-based sections
- Grid-based page layout with cell spanning and maximize
- Component data binding (typed output/input slots, wiring UI)
- First-party TextEditor, Spreadsheet, and Chart apps built in Dart to prove the SDK and data binding
- User/permission management (single-tenant), OIDC-compatible auth
- Jujutsu-backed storage with clean storage interface abstraction; read cache and permission index are stubbed behind the interface (swappable in phase 2 without application changes)
- Dart SDK: App manifest, Component protocol, Data binding API, VCS API, Agent CLI surface declaration
- Rust Agent CLI (`liquid` binary) — agents read/write through the same permission path as human users; all edits are attributed and reversible

Success criterion: a developer builds a Liquid app with data-binding components in Dart in under a day. An agent is provisioned, assigned a role, and performs versioned edits with a full audit trail — entirely via CLI.

**Phase 2 — Mobile + component protocol + multi-tenant + scale (6–12 months)**

- iOS and Android — same Flutter/Dart codebase from phase 1; Flutter's build tooling targets both with minimal delta
- Cross-app component protocol v1 (published, stable, versioned)
- Multi-tenant support; tenant-partitioned Jujutsu repositories
- Extension API
- Distributed read cache and materialized permission index deployed (replaces phase 1 stubs)
- Self-hosted registry with signed package verification
- Agent-to-agent data binding via CLI slot subscription

**Phase 3 — Ecosystem + HA**

- Multi-region / high-availability deployment guide
- Community app ecosystem opened
- Performance hardening: profiling at 10 000+ concurrent sessions, load testing, SLA documentation

---

## Conclusion

Liquid addresses real, under-served problems: VCS-native content, cross-app data-binding composability, agents as genuine first-class principals operating through a structured CLI, and a platform built for enterprise scale without SaaS lock-in.

Performance, security, scalability, and stability are not aspirational — they are design constraints enforced from phase 1. The key architectural decisions that make the scale targets achievable are all standard, proven patterns: content-addressed caching, a materialized permission index, tenant-partitioned storage, and a stateless request path. None of them are exotic; all of them must be designed for early so they can be swapped from stubs to production implementations in phase 2 without touching application code.

Flutter as the universal UI layer resolves the mobile question cleanly: one Dart codebase targets all five platforms through a consistent GPU-rendered pipeline. There is no WebView ceiling, no platform-specific rendering divergence, and no split SDK. Mobile arrives in phase 2 as a build target, not a separate workstream. The Rust ↔ Dart boundary via `flutter_rust_bridge` keeps all business logic, storage, and security in Rust, where it belongs.

The primary execution risk remains **scope**. The path to success is shipping a desktop v1 in Flutter that demonstrates the SDK, data binding, agent CLI, and VCS audit trail working together convincingly. That is the proof of concept that attracts contributors and validates the protocol before it is opened to the community.

With disciplined phasing, Liquid is feasible. Without it, it joins the long list of ambitious open-source platforms that never shipped.
