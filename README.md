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
   - [Components](#components)
   - [Extensions](#extensions)
   - [Tenants](#tenants)
   - [VCS (Jujutsu-native)](#vcs-jujutsu-native)
   - [User and Permission Management](#user-and-permission-management)
4. [SDK](#sdk)
5. [Technology Stack](#technology-stack)
6. [Self-Hosting](#self-hosting)
7. [Feasibility Assessment](#feasibility-assessment)
   - [What Is Technically Sound](#what-is-technically-sound)
   - [What Is Technically Novel and Hard](#what-is-technically-novel-and-hard)
   - [Risk Register](#risk-register)
   - [Competitive Landscape](#competitive-landscape)
   - [Recommended Phasing](#recommended-phasing)
8. [Conclusion](#conclusion)

---

## Vision

Liquid is a universal application platform — part UI framework, part SDK, part operating environment — that lets developers build rich, composable applications and lets users run, arrange, and govern those applications uniformly across Linux, Windows, macOS, iOS, and Android.

Where today's cross-platform stacks force trade-offs between native feel and developer ergonomics, Liquid targets both: a Rust core for performance and safety, a TypeScript surface for ecosystem reach, and a component model that is as composable as Notion blocks but without Notion's walled garden.

---

## Why Liquid?

| Problem today | Liquid's answer |
|---|---|
| Every app reinvents user management, roles, and permissions | First-class user/permission layer shared across all apps |
| Data loss from agents editing production content | VCS native — every change is versioned, reversible, and attributable |
| Mobile apps are second-class citizens in "cross-platform" tools | Mobile-first grid layout and rendering pipeline |
| Subscriptions required for core functionality | Fully self-hostable; no SaaS dependency |
| Components are siloed inside apps | Cross-app component protocol: any component works anywhere |
| Agent access is coarse-grained or uncontrolled | Permission scopes apply to human users and AI agents equally |

---

## Core Concepts

### Explorer

The left panel of the Liquid shell. Displays all available apps, components, and tagged content. Inspired by the sidebars in VS Code, Obsidian, and Notion, but driven by a structured tag and filter system:

- **Tags** — arbitrary labels attached to any entity (app, component, document)
- **Custom sections** — user-defined groupings backed by pattern-matching filter rules
- **Visibility rules** — hide or surface content based on tag combinations, ownership, or tenant scope

The explorer is fully user-configurable. Power users can replicate a VS Code–style file tree, a Notion-style page hierarchy, or a flat Obsidian-style tag cloud using the same underlying mechanism.

### Pages and the Grid

The right-hand content area is called a **page**. A page is divided by a responsive grid — conceptually similar to a mobile home screen or ClickUp's card view.

- Grid cells are resizable and reorderable
- Any app or component can be placed into a grid cell
- A cell can be maximized to fill the entire page
- Multiple cells can coexist on one page across different apps

Pages are first-class entities: they are versioned, ownable, and shareable.

### Apps

Apps are the primary unit of functionality in Liquid. They are analogous to mobile apps or ClickUp cards:

- Developed against the Liquid SDK
- Distributed through Liquid's open registry or self-hosted registries
- Rendered inside grid cells on a page
- Subject to the permission model of the tenant they run in

Apps are composites of components and have no special rendering privilege over components — they are organizational and distribution units, not rendering containers.

### Components

Components are the atomic building block of Liquid. Key design principles:

- **Cross-app compatible** — a `TextEditor` component developed for App A can be embedded in App B without modification
- **Same z-level rendering** — components placed adjacent to each other share the same rendering plane; there is no layering or iframe isolation between them
- **Shared event surface** — pointer, keyboard, and touch events are dispatched to all components that geometrically intersect the event origin; components negotiate handling via a priority protocol rather than z-index stacking
- **Example:** A whiteboard component and a text editor component placed side-by-side in the grid share the same canvas plane. Drawing on the area occupied by the text editor produces marks on both the text layer and the drawing layer simultaneously because both components subscribe to the same input events in that region.

This model requires a unified rendering and event coordination layer — one of Liquid's core technical contributions.

### Extensions

Extensions enrich existing apps and components without forking them:

- Hook into lifecycle events of any app or component they have permission to extend
- Can add UI surface, transform data, or intercept events
- Distributed and versioned independently from the host app

Extensions follow the same permission model as apps.

### Tenants

A single Liquid user can have multiple **tenants** — isolated environments with separate app configurations, data, and permission scopes:

- Example: one tenant for personal use, one for a work organization
- Switching tenants switches the full context: apps visible in the explorer, pages, and the user's effective permissions
- Agents assigned to a tenant inherit only that tenant's permission scope

### VCS (Jujutsu-native)

All content in Liquid — pages, components, documents, configuration — is stored in a Jujutsu repository. This is not an add-on; it is the storage layer.

- Every edit is a commit: no accidental data loss
- Agent edits are attributed to the agent's identity and are reversible
- Branching and merging are available to end users through the Liquid UI, not just the CLI
- Self-hosted sync via any Jujutsu-compatible remote (NAS, server, cloud)

Jujutsu was chosen over Git for its cleaner operation model, better conflict handling, and first-class support for operation log (undo of any operation, not just file changes).

### User and Permission Management

Liquid provides a unified identity and permission layer across all apps:

- Users are defined once per Liquid installation
- Permissions are scoped to: tenant → app → component → document → field
- AI agents are treated as first-class principals: an agent is assigned a role with explicit read/write scopes
- Audit log of all access is stored in the VCS operation log

---

## SDK

The Liquid SDK provides:

- **App manifest** — declare dependencies, required permissions, grid size constraints
- **Component protocol** — register components, declare event subscriptions, expose slots for extension
- **Grid API** — request layout changes, respond to resize/maximize events
- **VCS API** — read/write versioned content, access history, create branches
- **Permission API** — query effective permissions for the current user/agent
- **Extension API** — hook into other apps/components if permitted

Target languages: TypeScript (primary), with Rust bindings for performance-critical components.

---

## Technology Stack

| Layer | Technology | Rationale |
|---|---|---|
| Core runtime | Rust | Memory safety, performance, cross-platform compilation |
| UI rendering | To be determined (see risks) | WebView via Tauri, or native GPU via wgpu/iced |
| App/component logic | TypeScript | Ecosystem size, developer familiarity |
| VCS storage | Jujutsu | Operation log, cleaner conflict model vs. Git |
| Mobile shell | Tauri Mobile (experimental) or Flutter | Tauri for stack consistency; Flutter as fallback |
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
Tauri has proven that a Rust backend + TypeScript frontend can ship a production-quality cross-platform desktop application. This part of the stack is well-understood in 2025.

**Grid layout engine**
Responsive, drag-and-drop grid layouts are solved problems in web rendering (CSS Grid, react-grid-layout). Porting this to a Liquid shell is substantial work but not novel research.

**Multi-tenant user management**
Tenant isolation, RBAC, and per-tenant app configuration are standard enterprise software patterns. Well-documented, well-tested.

**VCS-backed storage**
Using a VCS as a content store is unconventional but not unprecedented (Obsidian with Git, Foam, etc.). Jujutsu's operation log makes this more ergonomic than Git. The main implementation work is building a high-level content API on top of Jujutsu's low-level operations.

**Extension system**
Hook-based extension architectures are mature (VS Code's extension API is the gold standard). Replicating a restricted version of this is achievable.

**Agent permission scoping**
Treating AI agents as first-class principals in an RBAC system is straightforward to implement once the RBAC model exists. The design insight is correct and valuable.

---

### What Is Technically Novel and Hard

**Same z-level component rendering (HIGH difficulty)**

This is the most technically ambitious claim in the spec. Standard UI frameworks assume a strict z-order: components are layered, and events are dispatched to the topmost layer first. Liquid's model — where a whiteboard and a text editor coexist at the same z-level and both receive paint events — requires:

1. A unified compositing layer that does not use z-index for hit-testing
2. A spatial event router that finds all components intersecting an input event and broadcasts it
3. A component-level negotiation protocol for conflicting event claims
4. A rendering model where partial transparency and blending between co-planar components is defined behavior

No existing UI framework provides this. It will need to be built from scratch or the semantics will need to be narrowed significantly (e.g., "same z-level" means side-by-side in the grid, not overlapping pixels, which is a simpler model).

**Mobile-native with Rust core (HIGH difficulty)**

Tauri Mobile as of 2025 is still experimental. iOS and Android development with a Rust/WebView stack has significant surface area for platform-specific breakage (WKWebView quirks on iOS, WebView versioning on Android). Flutter avoids most of this but introduces a second language (Dart) and diverges from the Rust/TypeScript stack.

This is the single highest-risk platform target.

**Jujutsu ecosystem maturity (MEDIUM difficulty)**

Jujutsu is under active development. Its library API (jj-lib) is not yet stable. Building a production system on top of an unstable library API means accepting ongoing migration cost as Jujutsu evolves. Git would be more stable but operationally inferior for the use cases Liquid needs.

**Cross-app component protocol (MEDIUM-HIGH difficulty)**

For a component built by Developer A to run correctly inside an app built by Developer B, both must conform to a shared protocol for:
- DOM/render tree ownership
- Event propagation
- Layout negotiation
- State management
- Security sandboxing (a malicious component must not access another component's data)

Defining and stabilizing this protocol is a significant design and standardization effort before any app ecosystem can form.

---

### Risk Register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Mobile targets (iOS/Android) delayed or cut | High | Medium | Treat mobile as phase 2; ship desktop first |
| Same z-level component model proves too complex | Medium | High | Narrow to side-by-side grid cells (no pixel overlap) in v1 |
| Jujutsu API instability causes rework | Medium | Medium | Pin jj-lib version; track upstream; budget migration time |
| Cross-app component protocol fails to attract third-party developers | Medium | High | Ship 5–10 first-party apps to prove the protocol before opening |
| Tauri Mobile not production-ready when needed | High | Medium | Evaluate Flutter as fallback; keep rendering layer swappable |
| Scope creep kills momentum | High | High | Phase strictly; cut features ruthlessly in v1 |
| Open-source contributor acquisition | Medium | Medium | Strong developer docs and a compelling demo early |
| Security vulnerabilities in extension sandboxing | Medium | High | Capability-based permissions; security audit before public registry |
| Competing with Notion/ClickUp/Flutter brand recognition | High | Low | Liquid is a framework, not a SaaS; different market |

---

### Competitive Landscape

| Project | Overlap | Key difference |
|---|---|---|
| **Tauri** | Cross-platform Rust + TS desktop | Tauri is a shell; Liquid is a full application platform with component protocol, VCS, and user management |
| **Flutter** | Cross-platform including mobile | Dart ecosystem, no VCS integration, no component protocol |
| **Electron** | Cross-platform desktop | Heavy, Chromium bundled, no VCS, no component protocol |
| **Notion** | UX inspiration, block model | Closed, SaaS, no SDK, no VCS, no developer framework |
| **Obsidian** | Explorer UX, extensibility | Note-taking only, no cross-app component protocol, Git plugin not native |
| **ClickUp** | Card/widget layout inspiration | SaaS, closed ecosystem, no component protocol |
| **AppFlowy** | Open Notion alternative | No cross-platform mobile, no component SDK, no VCS |

Liquid's combination of **open SDK + cross-app components + VCS-native + agent-ready permissions** has no direct equivalent. The risk is not that competitors exist but that the scope is so large it is hard to reach a compelling v1.

---

### Recommended Phasing

**Phase 1 — Desktop shell + SDK foundation (12–18 months, small team)**

- Liquid shell for Linux, Windows, macOS (Tauri)
- Explorer panel with tags and custom sections
- Grid-based page layout (side-by-side cells, no pixel overlap between components)
- First-party TextEditor and Kanban apps to prove the SDK
- Basic user/permission management (single-tenant)
- Jujutsu-backed storage for pages and documents
- TypeScript SDK with App manifest, Component protocol, VCS API

Success criterion: a developer can build and ship a Liquid app in under a day using the SDK.

**Phase 2 — Component protocol + multi-tenant (6–12 months)**

- Cross-app component protocol v1 (published, stable)
- Multi-tenant support
- Extension API
- Agent permission scoping
- Self-hosted registry

**Phase 3 — Mobile + same z-level rendering research**

- iOS and Android (Tauri Mobile or Flutter bridge)
- Prototype the same z-level event model in isolation; promote to stable only if semantics can be clearly specified and implemented
- Community app ecosystem

---

## Conclusion

Liquid addresses real, under-served problems: VCS-native content, cross-app composability, agent-aware permissions, and a powerful mobile-native platform. The vision is coherent and the market gap is genuine.

The primary feasibility risk is **scope**. The project as described is equivalent in scale to building VS Code, Notion, a new UI rendering pipeline, a VCS integration layer, and a mobile framework — simultaneously, from scratch, as open source. No single team ships all of that in v1.

The path to success is aggressive phasing: build a compelling desktop-only v1 that proves the SDK and component protocol, attract contributors and app developers, and expand to mobile and the more exotic rendering features only after the core is stable and adopted.

The same z-level component model is the most technically risky single idea. It should be treated as a long-term research goal, not a v1 requirement. A grid that places components side-by-side (not overlapping) is nearly as powerful for most use cases and is achievable in phase 1.

With disciplined scoping, Liquid is feasible. Without it, it joins the long list of ambitious open-source platforms that never shipped.
