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
   - [Running Liquid on Mobile: Tauri Mobile vs Flutter](#running-liquid-on-mobile-tauri-mobile-vs-flutter)
   - [Risk Register](#risk-register)
   - [Competitive Landscape](#competitive-landscape)
   - [Recommended Phasing](#recommended-phasing)
9. [Conclusion](#conclusion)

---

## Vision

Liquid is a universal application platform — part UI framework, part SDK, part operating environment — that lets developers build rich, composable applications and lets users run, arrange, and govern those applications uniformly across Linux, Windows, macOS, iOS, and Android.

Where today's cross-platform stacks force trade-offs between native feel and developer ergonomics, Liquid targets both: a Rust core for performance and safety, a TypeScript surface for ecosystem reach, and a component model that is as composable as Notion blocks but without Notion's walled garden.

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

Target languages: TypeScript (primary), with Rust bindings for performance-critical components.

---

## Technology Stack

| Layer | Technology | Rationale |
|---|---|---|
| Core runtime | Rust | Memory safety, performance, cross-platform compilation |
| Desktop UI shell | Tauri 2.x (WebView) | Proven Rust + TypeScript cross-platform desktop; production-stable |
| App/component logic | TypeScript | Ecosystem size, developer familiarity |
| VCS storage | Jujutsu | Operation log, cleaner conflict model vs. Git, large-repo performance |
| Mobile shell | Tauri Mobile or Flutter (decision in phase 2) | See [Running Liquid on Mobile](#running-liquid-on-mobile-tauri-mobile-vs-flutter) |
| Read cache | Redis-class distributed cache | Sub-millisecond warm reads; content-addressed = exact invalidation |
| Permission index | Materialized key-value store | Single-lookup permission checks at 20 000+ concurrent principals |
| Event bus | Kafka-class message bus | Fan-out for multi-region replication and data binding at scale |
| Agent interface | Liquid Agent CLI (generated from app manifest) | Structured, scriptable, zero rendering overhead |
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

**Cross-platform desktop shell (Linux, Windows, macOS)**
Tauri 2.x is production-stable. A Rust backend + TypeScript frontend cross-platform desktop application is a well-proven pattern in 2026.

**Grid layout engine**
Responsive grid layouts with spanning and drag-and-drop are solved problems in web rendering. The static-grid-with-spanning model Liquid uses is simpler than fully freeform canvas layouts and straightforward to implement correctly.

**Component data binding**
Typed publish/subscribe between components is a well-understood pattern (RxJS, spreadsheet cell references, Unix pipes). Implementing it as a first-class SDK primitive is architecturally novel for a UI framework but not technically risky.

**Multi-tenant user management**
Tenant isolation, RBAC, and per-tenant app configuration are standard enterprise software patterns. Well-documented, well-tested.

**VCS-backed storage with caching**
Using a VCS as a content store is unconventional but sound. Jujutsu's operation log and content-addressed object model are particularly well-suited: objects are immutable by hash, making a Redis-class read cache trivially correct. Every major collaborative platform (Figma, Notion, Linear) uses a write-ahead log as durable storage with a caching layer for hot reads — Liquid's architecture follows the same proven pattern.

**Enterprise-scale permission system**
A materialized permission index (RBAC evaluated once on policy change, results stored for O(1) lookup) is the standard approach for fine-grained permissions at tens of thousands of concurrent principals. This is operationally non-trivial but architecturally well-understood.

**Agent-as-principal with CLI interface**
Treating AI agents as first-class RBAC principals with a generated CLI surface is straightforward to implement once the RBAC model and app manifest format exist. The CLI generation pattern (manifest → commands) is the same approach used by tools like the AWS CLI and Kubernetes kubectl.

**Extension system**
Hook-based extension architectures are mature (VS Code's extension API is the gold standard). Replicating a restricted version of this is achievable.

---

### What Is Technically Novel and Hard

**Cross-app component protocol (MEDIUM-HIGH difficulty)**

For a component built by Developer A to run correctly inside an app built by Developer B, both must conform to a shared protocol for DOM/render tree ownership, event propagation, layout negotiation, data slot types, and security sandboxing. Defining and stabilizing this protocol is a significant design effort before any app ecosystem can form. Shipping 5–10 first-party apps that exercise the protocol is the only way to validate it before opening it to third parties.

**Scalable VCS + caching layer (MEDIUM difficulty)**

Achieving millisecond latency at enterprise scale requires a caching and indexing layer in front of Jujutsu (see Core Design Principles). The architecture is standard, but the interface between the application layer and the storage/cache layer must be defined cleanly in phase 1. Retrofitting it later means rewriting application code. The effort is non-trivial but the approach is well-understood.

**Jujutsu ecosystem maturity (MEDIUM difficulty)**

Jujutsu's library API (jj-lib) is not yet stable. Building a production system on top of it means accepting ongoing migration cost as Jujutsu evolves. Git would be more stable but operationally inferior for the use cases Liquid needs.

---

### Running Liquid on Mobile: Tauri Mobile vs Flutter

**The core question: how does Liquid get onto a phone at all?**

On Linux, Windows, and macOS, Tauri handles packaging. It wraps the Rust core and the TypeScript UI into a native desktop window that the OS can launch — no browser needed, no Electron-style bundled Chromium. The result is a small, fast native app.

On iOS and Android, the situation is different. Apple and Google each have their own app ecosystems with strict rules: to distribute on the App Store or Play Store, the code must be packaged as a native app in a format they accept. You cannot just ship a web page or a Linux binary. This means Liquid needs a separate packaging and runtime strategy for mobile — and that is where Tauri Mobile and Flutter come in.

**Option A: Tauri Mobile**

Tauri Mobile (introduced in Tauri v2.0, stable since October 2024) extends Tauri's model to iOS and Android. The Rust core compiles to a native library; the TypeScript UI runs inside the platform's built-in WebView (WKWebView on iOS, Android System WebView on Android). One codebase, five targets.

*Current status (2026):* Tauri Mobile is production-ready for straightforward apps. Native APIs for biometric auth, notifications, NFC, clipboard, and deep links are available. The mobile targets reached stable API status in v2.0.0. The gap relative to the desktop version is that not all desktop plugins have been ported to mobile yet, and the Tauri team has described v2 mobile as "a solid foundation" rather than feature-complete.

*Limitations that matter for Liquid:*

- **WebView fragmentation** — on iOS, Apple mandates WKWebView for all third-party apps; no alternative engine is permitted. WKWebView's CSS and JavaScript behavior is tied to the iOS version. On Android, the WebView is the system Chromium and varies by device and Android version.
- **Native UX feel** — swipe gestures, momentum scrolling, haptic feedback, and platform-specific animations that users expect on iOS and Android are not automatic inside a WebView. Reproducing them requires significant CSS and JavaScript work, and the results can still fall short of a fully native feel.
- **Performance ceiling** — mobile hardware is meaningfully weaker than desktop. A WebView rendering pipeline adds overhead. A grid with multiple live data-binding components needs careful optimization to stay smooth at 60 fps.

**Option B: Flutter**

Flutter (v3.38, November 2025 — now in its "Production Era") does not use a WebView at all. It renders everything through its own GPU-accelerated pipeline called Impeller, which is fully stable on iOS and Android as of Flutter 3.22 and delivers consistent 60/120 fps across devices without shader compilation jank. Flutter compiles to native ARM binaries and communicates directly with platform APIs.

*Trade-off:* Flutter requires Dart as a fourth language (alongside Rust, TypeScript, and any Swift/Kotlin for native plugins). It diverges from Liquid's core stack. However, with a clean Rust core library, the Flutter shell can call into the same Rust business logic via FFI — it only replaces the rendering and packaging layer.

**Comparison**

| | Tauri Mobile | Flutter |
|---|---|---|
| Language overhead | None (same TS/Rust) | Dart (new language) |
| Rendering | Platform WebView | Custom GPU pipeline (Impeller) |
| Native UX feel | Requires CSS/JS effort | Native by default |
| Mobile performance | Good, WebView-limited | Excellent, no WebView |
| Production status (2026) | Stable, feature gaps vs desktop | Production Era, fully stable |
| Desktop support | Yes (same codebase) | Yes but less mature than mobile |

**Recommendation**

Treat mobile as phase 2. In phase 1, ship and validate the desktop experience, SDK, and agent CLI. Before committing to either mobile approach, run a 2–3 week spike: implement the Liquid grid layout and one data-binding component pair on a real iOS and Android device using Tauri Mobile. Measure frame rate, gesture responsiveness, and overall feel. If the result meets the quality bar, proceed with Tauri Mobile (zero stack overhead). If it does not, adopt Flutter with a shared Rust core for the business logic layer.

---

### Risk Register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Mobile UX quality bar not met with Tauri Mobile WebView | Medium | Medium | Run phase 2 spike on real devices; keep Flutter as a drop-in alternative for the rendering layer only |
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
| **Tauri** | Cross-platform Rust + TS desktop | Tauri is a shell; Liquid is a full application platform with component protocol, VCS, and user management |
| **Flutter** | Cross-platform including mobile | Dart ecosystem, no VCS integration, no component protocol |
| **Electron** | Cross-platform desktop | Heavy, Chromium bundled, no VCS, no component protocol |
| **Notion** | UX inspiration, block model, page tree | Closed, SaaS, no SDK, no VCS, no developer framework |
| **Obsidian** | Explorer UX, extensibility | Note-taking only, no cross-app component protocol, Git plugin not native |
| **ClickUp** | Card/widget layout inspiration | SaaS, closed ecosystem, no component protocol |
| **AppFlowy** | Open Notion alternative | No cross-platform mobile, no component SDK, no VCS |

Liquid's combination of **open SDK + cross-app data-binding components + VCS-native + agent-as-principal + enterprise scale** has no direct equivalent. The risk is not that competitors exist but that the scope is so large it is hard to reach a compelling v1.

---

### Recommended Phasing

**Phase 1 — Desktop shell + SDK foundation (12–18 months, small team)**

- Liquid shell for Linux, Windows, macOS (Tauri 2.x)
- Explorer panel with page tree (icons, subpages, drag-and-drop) and tag-based sections
- Grid-based page layout with cell spanning and maximize
- Component data binding (typed output/input slots, wiring UI)
- First-party TextEditor, Spreadsheet, and Chart apps to prove the SDK and data binding
- User/permission management (single-tenant), OIDC-compatible auth
- Jujutsu-backed storage with clean storage interface abstraction; read cache and permission index are stubbed behind the interface (implementation can be in-process for phase 1, swapped for distributed in phase 2 without application changes)
- TypeScript SDK: App manifest, Component protocol, Data binding API, VCS API, Agent CLI surface generator
- Agent identity, authentication, and CLI — agents can read/write via `liquid` CLI through the same permission path as human users; all edits are attributed and reversible

Success criterion: a developer builds a Liquid app with data-binding components in under a day. An agent is provisioned, assigned a role, and performs versioned edits with a full audit trail — entirely via CLI.

**Phase 2 — Component protocol + multi-tenant + scale (6–12 months)**

- Cross-app component protocol v1 (published, stable, versioned)
- Multi-tenant support; tenant-partitioned Jujutsu repositories
- Extension API
- Distributed read cache and materialized permission index deployed (replaces phase 1 stubs)
- Self-hosted registry with signed package verification
- Agent-to-agent data binding via CLI slot subscription
- Mobile spike: build grid + one data-binding pair on real iOS/Android devices with Tauri Mobile; decide Tauri Mobile vs Flutter

**Phase 3 — Mobile + ecosystem + HA**

- iOS and Android (Tauri Mobile if spike passed quality bar; Flutter otherwise)
- Multi-region / high-availability deployment guide
- Community app ecosystem opened
- Performance hardening: profiling at 10 000+ concurrent sessions, load testing, SLA documentation

---

## Conclusion

Liquid addresses real, under-served problems: VCS-native content, cross-app data-binding composability, agents as genuine first-class principals operating through a structured CLI, and a platform built for enterprise scale without SaaS lock-in.

Performance, security, scalability, and stability are not aspirational — they are design constraints enforced from phase 1. The key architectural decisions that make the scale targets achievable are all standard, proven patterns: content-addressed caching, a materialized permission index, tenant-partitioned storage, and a stateless request path. None of them are exotic; all of them must be designed for early so they can be swapped from stubs to production implementations in phase 2 without touching application code.

The mobile question is no longer a binary between "Tauri Mobile is experimental" and "rewrite in Flutter." Tauri 2.x mobile is production-stable as of 2026. The remaining question is whether WebView-based rendering meets Liquid's quality bar for a mobile-first experience — and that is answered by a focused spike in phase 2, not by assumption.

The primary execution risk remains **scope**. The path to success is shipping a desktop v1 that demonstrates the SDK, data binding, agent CLI, and VCS audit trail working together convincingly. That is the proof of concept that attracts contributors and validates the protocol before it is opened to the community.

With disciplined phasing, Liquid is feasible. Without it, it joins the long list of ambitious open-source platforms that never shipped.
