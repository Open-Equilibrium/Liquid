# Liquid

> An open-source, cross-platform UI framework and SDK for building apps, components, and extensions — with VCS, user management, and agent-ready permissions built in from day one.

---

## Table of Contents

1. [Vision](#vision)
2. [Why Liquid?](#why-liquid)
3. [Core Concepts](#core-concepts)
   - [Explorer](#explorer)
   - [Pages and the Grid](#pages-and-the-grid)
   - [Apps](#apps)
   - [Components and Data Binding](#components-and-data-binding)
   - [Extensions](#extensions)
   - [Tenants](#tenants)
   - [VCS (Jujutsu-native)](#vcs-jujutsu-native)
   - [User and Permission Management](#user-and-permission-management)
   - [Agents as First-Class Citizens](#agents-as-first-class-citizens)
4. [SDK](#sdk)
5. [Technology Stack](#technology-stack)
6. [Self-Hosting](#self-hosting)
7. [Scalability at Enterprise Scale](#scalability-at-enterprise-scale)
8. [Feasibility Assessment](#feasibility-assessment)
   - [What Is Technically Sound](#what-is-technically-sound)
   - [What Is Technically Novel and Hard](#what-is-technically-novel-and-hard)
   - [Tauri Mobile: What It Is and Why Mobile-First Is Hard](#tauri-mobile-what-it-is-and-why-mobile-first-is-hard)
   - [Risk Register](#risk-register)
   - [Competitive Landscape](#competitive-landscape)
   - [Recommended Phasing](#recommended-phasing)
9. [Conclusion](#conclusion)

---

## Vision

Liquid is a universal application platform — part UI framework, part SDK, part operating environment — that lets developers build rich, composable applications and lets users run, arrange, and govern those applications uniformly across Linux, Windows, macOS, iOS, and Android.

Where today's cross-platform stacks force trade-offs between native feel and developer ergonomics, Liquid targets both: a Rust core for performance and safety, a TypeScript surface for ecosystem reach, and a component model that is as composable as Notion blocks but without Notion's walled garden.

Agents are not an afterthought. Liquid is designed from the ground up for environments where AI agents and human users collaborate at equal standing — sharing the same permission model, the same VCS audit trail, and the same UI surface.

---

## Why Liquid?

| Problem today | Liquid's answer |
|---|---|
| Every app reinvents user management, roles, and permissions | First-class user/permission layer shared across all apps |
| Data loss from agents editing production content | VCS native — every change is versioned, reversible, and attributable |
| Mobile apps are second-class citizens in "cross-platform" tools | Mobile-first grid layout and rendering pipeline |
| Subscriptions required for core functionality | Fully self-hostable; no SaaS dependency |
| Components are siloed inside apps, data cannot flow between them | Cross-app component data binding: any component can publish and consume typed data streams |
| Agent access is coarse-grained or uncontrolled | Agents are first-class principals with the same fine-grained permission model as human users |
| Frameworks are not built for enterprise scale | Designed for 10 000+ users, 10 000+ agents, and millions of files with millisecond operation latency |

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

Agents are not plugins, integrations, or privileged daemons in Liquid. They are principals — equal in standing to human users — and the entire framework is designed with this in mind.

**Identity**

Every agent has a unique identity within a tenant. It authenticates the same way a human user does and is subject to the same session and token management.

**Permissions**

Agents receive roles and permission scopes through the same RBAC system as humans:

- An agent can be granted read access to specific pages, components, or data fields and nothing else
- An agent cannot exceed the permissions of the human who authorized it
- Permission changes take effect immediately and are reflected in the VCS audit log

**UI surface**

Agents are not limited to API access. An agent can:

- Render its own components on a page (e.g., a summary card, a status widget, a generated chart)
- Subscribe to component data slots and react to user-driven changes in real time
- Write back to components it has write permission for, with every edit attributed and reversible

**Agent-to-agent collaboration**

Multiple agents can be assigned to the same tenant and page. They can communicate through the same data binding system components use — an agent publishes to a slot, another agent (or a component) subscribes to it. No special inter-agent API is needed.

**Audit and reversibility**

Because all writes go through the VCS layer, every action an agent takes is:

- Attributed to that agent's identity
- Timestamped
- Reversible with a single undo operation
- Visible in the operation log alongside human edits

This makes agent work safe enough to allow in production environments, which is a primary design goal of Liquid.

---

## SDK

The Liquid SDK provides:

- **App manifest** — declare dependencies, required permissions, grid size constraints
- **Component protocol** — register components, declare input/output data slots, expose extension hooks
- **Grid API** — request layout changes, respond to resize/maximize events
- **Data binding API** — publish to output slots, subscribe to input slots, define slot types
- **VCS API** — read/write versioned content, access history, create branches
- **Permission API** — query effective permissions for the current user or agent
- **Agent API** — register agent identity, declare capabilities, manage lifecycle
- **Extension API** — hook into other apps/components if permitted

Target languages: TypeScript (primary), with Rust bindings for performance-critical components.

---

## Technology Stack

| Layer | Technology | Rationale |
|---|---|---|
| Core runtime | Rust | Memory safety, performance, cross-platform compilation |
| UI rendering | WebView via Tauri | Proven cross-platform approach; see mobile discussion below |
| App/component logic | TypeScript | Ecosystem size, developer familiarity |
| VCS storage | Jujutsu | Operation log, cleaner conflict model vs. Git, large-repo performance |
| Mobile shell | Tauri Mobile (experimental) | See [Tauri Mobile section](#tauri-mobile-what-it-is-and-why-mobile-first-is-hard) |
| Data layer (enterprise) | Content-addressable store over Jujutsu + optional distributed cache | Millisecond reads at scale require a caching layer above raw VCS |
| Package registry | Self-hosted, open protocol | No vendor lock-in |

---

## Self-Hosting

Liquid is designed to run without any external subscription:

- The Liquid shell and registry can be deployed on a personal server or NAS
- VCS remotes are standard Jujutsu remotes (SSH, HTTP)
- User management is local or federated (OIDC-compatible)
- No telemetry by default

---

## Scalability at Enterprise Scale

One of Liquid's explicit design targets is an organization with **10 000+ human users, 10 000+ agents, and millions of files** — all operating concurrently, with operations completing in milliseconds. This section examines whether that target is achievable and what architecture it requires.

### The Challenge

A VCS-backed content store is not a traditional database. Jujutsu (and Git underneath it) is optimized for correctness and history, not for low-latency random reads at high concurrency. Naively querying a Jujutsu repository for a single file at the scale of millions of objects and tens of thousands of concurrent users would be slow.

The permission system faces a similar challenge: evaluating fine-grained per-field permissions for 20 000 simultaneous principals on every read/write is expensive if done naively.

### Architecture for Scale

Liquid's scalability strategy relies on three layers:

**1. Content-addressable cache**

Raw VCS objects are immutable once written (content-addressed by hash). This property makes them trivially cacheable:

- A distributed read cache (Redis-class or equivalent) stores recently accessed objects by hash
- Cache invalidation is exact: when a commit changes an object, only that hash is evicted
- Cold reads hit Jujutsu; warm reads (the overwhelming majority in steady state) hit the cache in sub-millisecond time
- The cache is horizontally scalable — add nodes as the user base grows

**2. Permission index**

Rather than evaluating the full RBAC graph on every operation, Liquid maintains a materialized permission index:

- The index is updated asynchronously whenever roles or permissions change
- Read checks are a single key lookup against the index
- Write operations validate against the index before committing to the VCS layer
- Index updates propagate within a configurable consistency window (e.g., < 1 second)

**3. Event-driven replication**

For multi-region or high-availability deployments:

- A change event bus (Kafka-class or equivalent) fans out VCS commits to replica nodes
- Each replica maintains its own cache and permission index
- Reads are served locally; writes go to the primary and propagate asynchronously

### Feasibility Verdict

| Target | Feasible? | Condition |
|---|---|---|
| 10 000 concurrent human users | Yes | Standard web-scale architecture; well understood |
| 10 000 concurrent agents | Yes | Agents use the same auth/permission path as humans; stateless request handling scales horizontally |
| Millions of files | Yes | Content-addressable storage scales to this; Git/Jujutsu already handles monorepos of this size (see Google's internal tooling) |
| Millisecond operation latency | **Conditionally yes** | Requires the caching layer described above; raw VCS reads are not millisecond at this scale without it |
| Fine-grained per-field permissions at scale | Yes | Requires the materialized permission index; not feasible with live graph traversal |

**The honest constraint:** millisecond latency at this scale is achievable but it is a significant systems engineering effort on top of the application layer. It requires treating the VCS as a durable write-ahead log and source of truth, not as the hot read path. The caching and indexing layers add operational complexity and are non-trivial to build correctly.

This is not a reason to abandon the goal — every major collaborative platform (Figma, Notion, Linear) solves essentially the same problem. It is a reason to design the storage and permission interfaces cleanly in phase 1 so that the caching layer can be inserted later without rewriting the application logic.

---

## Feasibility Assessment

### What Is Technically Sound

**Cross-platform desktop shell (Linux, Windows, macOS)**
Tauri has proven that a Rust backend + TypeScript frontend can ship a production-quality cross-platform desktop application. This part of the stack is well-understood in 2025.

**Grid layout engine**
Responsive grid layouts with spanning and drag-and-drop are solved problems in web rendering. The static-grid-with-spanning model Liquid uses is simpler than fully freeform canvas layouts and is straightforward to implement correctly.

**Component data binding**
Typed publish/subscribe between components is a well-understood pattern (think RxJS, spreadsheet cell references, or Unix pipes). Implementing it as a first-class SDK primitive is new in this context but not technically risky.

**Multi-tenant user management**
Tenant isolation, RBAC, and per-tenant app configuration are standard enterprise software patterns. Well-documented, well-tested.

**VCS-backed storage**
Using a VCS as a content store is unconventional but not unprecedented (Obsidian with Git, Foam, etc.). Jujutsu's operation log makes this more ergonomic than Git. The main implementation work is building a high-level content API on top of Jujutsu's low-level operations.

**Agent-as-principal model**
Treating AI agents as first-class RBAC principals is straightforward to implement once the RBAC model exists. The design insight is correct and forward-looking.

**Extension system**
Hook-based extension architectures are mature (VS Code's extension API is the gold standard). Replicating a restricted version of this is achievable.

---

### What Is Technically Novel and Hard

**Cross-app component protocol (MEDIUM-HIGH difficulty)**

For a component built by Developer A to run correctly inside an app built by Developer B, both must conform to a shared protocol for DOM/render tree ownership, event propagation, layout negotiation, data slot types, and security sandboxing. Defining and stabilizing this protocol is a significant design effort before any app ecosystem can form. Shipping 5–10 first-party apps that exercise the protocol is the only way to validate it before opening it to third parties.

**Scalable VCS + caching layer (MEDIUM difficulty)**

As described in the scalability section, achieving millisecond latency at enterprise scale requires an explicit caching and indexing layer on top of Jujutsu. This is architecturally standard but non-trivial to implement and operate. It must be designed for in phase 1 even if it is not fully implemented until phase 2.

**Jujutsu ecosystem maturity (MEDIUM difficulty)**

Jujutsu's library API (jj-lib) is not yet stable. Building a production system on top of it means accepting ongoing migration cost as Jujutsu evolves. Git would be more stable but operationally inferior for the use cases Liquid needs.

---

### Tauri Mobile: What It Is and Why Mobile-First Is Hard

**What is Tauri Mobile?**

Tauri is a framework for building desktop applications using a Rust backend and a web-based frontend (HTML/CSS/TypeScript). The desktop version is mature and production-ready. **Tauri Mobile** is Tauri's extension of this model to iOS and Android: the same Rust core and TypeScript UI, packaged as a native iOS app (using WKWebView) or an Android app (using the system WebView).

Tauri Mobile lets developers write one codebase and compile it to Linux, Windows, macOS, iOS, and Android. On paper this is exactly what Liquid needs.

**Why mobile-first is hard with Tauri Mobile**

*1. Tauri Mobile is still experimental (as of 2025)*
The mobile target was introduced in Tauri v2 and has not yet reached the same stability level as the desktop target. APIs are still changing, and the number of production apps shipping with it is small. Relying on it as the foundation for a mobile-first product means accepting upstream instability.

*2. WebView fragmentation on mobile*
On iOS, all WebViews must use Apple's WKWebView engine — apps cannot bundle their own browser engine (unlike desktop, where Tauri bundles the OS WebView but Chrome/Firefox don't run as WebViews). This means:
- CSS and JavaScript behavior is tied to the iOS version the user is running
- Web APIs available on WKWebView lag behind those available in desktop browsers
- Performance-sensitive rendering (animations, large grids, canvas) can behave differently than expected

On Android, the WebView is the system Chromium, which varies by Android version and device manufacturer. In practice, Android WebView is more capable than WKWebView but still not identical to a native rendering pipeline.

*3. Mobile UX expectations are fundamentally different*
Native iOS and Android apps are built with platform-specific UI primitives (UIKit/SwiftUI on iOS, Jetpack Compose on Android) that provide gestures, animations, and interactions users expect — swipe-to-dismiss, momentum scrolling, haptic feedback, pull-to-refresh. Reproducing all of these faithfully inside a WebView requires significant custom CSS and JavaScript, and the results rarely feel fully native.

*4. Performance envelope*
Mobile CPUs and GPUs are significantly less powerful than their desktop counterparts. A grid-based layout with multiple live components, real-time data bindings, and VCS-backed storage needs to be carefully optimized for mobile. A WebView rendering pipeline adds overhead that a native rendering pipeline (Flutter's Impeller, for example) avoids.

**Alternative: Flutter**

Flutter renders using its own GPU-accelerated pipeline (Impeller on iOS/Android), which sidesteps WebView fragmentation entirely and delivers consistently native-feeling performance. The trade-off is adopting Dart as a second language alongside Rust and TypeScript, which diverges from Liquid's core stack.

**Recommendation**

Treat mobile as a phase 2 or phase 3 target. In phase 1, prove the desktop experience and SDK. Before committing to Tauri Mobile, run a 2–3 week technical spike: build the grid layout and one data-binding component on iOS and Android, measure rendering performance, and assess how much of the expected UX can be achieved in a WebView. If the spike results are poor, evaluate Flutter as a mobile-only frontend with a shared Rust core.

---

### Risk Register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Mobile targets (iOS/Android) delayed or cut | High | Medium | Treat mobile as phase 2; run technical spike before committing |
| Cross-app component protocol fails to attract third-party developers | Medium | High | Ship 5–10 first-party apps to prove the protocol before opening |
| Jujutsu API instability causes rework | Medium | Medium | Pin jj-lib version; track upstream; budget migration time |
| Caching/indexing layer under-designed in phase 1, requires rewrite | Medium | High | Define clean storage and permission interfaces early; stub the cache layer |
| Tauri Mobile not production-ready when needed | High | Medium | Evaluate Flutter as fallback; keep rendering layer swappable |
| Scope creep kills momentum | High | High | Phase strictly; cut features ruthlessly in v1 |
| Open-source contributor acquisition | Medium | Medium | Strong developer docs and a compelling demo early |
| Security vulnerabilities in extension sandboxing | Medium | High | Capability-based permissions; security audit before public registry |
| Agent identity spoofing or privilege escalation | Medium | High | Treat agent auth as a first-class security surface from day one |

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

- Liquid shell for Linux, Windows, macOS (Tauri)
- Explorer panel with page tree (icons, subpages, drag-and-drop) and tag-based sections
- Grid-based page layout with spanning and maximize
- Component data binding (typed output/input slots, wiring UI)
- First-party TextEditor, Spreadsheet, and Chart apps to prove the SDK and data binding
- Basic user/permission management (single-tenant)
- Jujutsu-backed storage with clean storage interface (cache layer stubbed)
- TypeScript SDK with App manifest, Component protocol, Data binding API, VCS API
- Agent identity and permissions (agents can read/write through the same auth path as humans)

Success criterion: a developer can build and ship a Liquid app with data-binding components in under a day. An agent can be assigned to a tenant and perform versioned edits with a full audit trail.

**Phase 2 — Component protocol + multi-tenant + scale (6–12 months)**

- Cross-app component protocol v1 (published, stable)
- Multi-tenant support
- Extension API
- Materialized permission index and read cache (enterprise scale targets)
- Self-hosted registry
- Agent-to-agent data binding and collaboration patterns

**Phase 3 — Mobile + ecosystem**

- iOS and Android (decision between Tauri Mobile and Flutter based on phase 1 spike results)
- Community app ecosystem
- Multi-region / high-availability deployment guide

---

## Conclusion

Liquid addresses real, under-served problems: VCS-native content, cross-app data-binding composability, agent-aware permissions at parity with human users, and a scalable enterprise platform without SaaS lock-in. The vision is coherent and the market gap is genuine.

The scalability target — 10 000+ users, 10 000+ agents, millions of files, millisecond latency — is achievable but requires deliberate architecture: a content-addressable read cache and a materialized permission index sitting in front of the VCS layer. These are not exotic components, but they must be designed for from phase 1, not retrofitted later.

The primary execution risk remains **scope**. The project as described is equivalent in scale to building VS Code, Notion, an enterprise identity system, and a VCS integration layer simultaneously. The path to success is aggressive phasing: ship a desktop-only v1 that proves the SDK, data binding model, and agent-as-principal design; attract contributors; and expand to mobile and enterprise scale only after the core is stable.

With disciplined scoping, Liquid is feasible. Without it, it joins the long list of ambitious open-source platforms that never shipped.
