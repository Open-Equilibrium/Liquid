# Liquid

> Cross-platform UI framework with VCS, fine-grained permissions, and AI agents as first-class principals — built in Rust + Flutter, scaling per workspace to 10 000+ users and 10 000+ agents.

[![CI](https://github.com/open-equilibrium/liquid/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/open-equilibrium/liquid/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Conventional Commits](https://img.shields.io/badge/Conventional%20Commits-1.0.0-yellow.svg)](https://www.conventionalcommits.org)

Liquid is an open-source platform — part UI framework, part SDK, part operating
environment — that lets developers build composable applications and lets users
arrange and govern those applications uniformly across **Linux, Windows, macOS,
iOS, and Android**. A Rust core handles versioning, permissions, and identity;
a Flutter/Dart shell handles rendering and input. AI agents share the same
identity model, permissions, and audit trail as human users, but interact
through a structured CLI rather than a GUI.

## Status

**Pre-alpha — Phase 1 in progress.** Liquid is under active early-stage
development. The core Rust crates are taking shape; the Flutter shell, public
SDK, and reference apps land in subsequent phases.

| Milestone | What ships | Status |
|---|---|---|
| **M1** Workspace + `liquid-core` primitives | `WorkspaceId`, `PrincipalId`, `LiquidError`, … | ✅ Done |
| **M2** VCS layer | `ContentStore` trait + `InMemory` + `Filesystem` impls | ✅ Done (jj-lib backend deferred — see [ADR-001](docs/adr/001-jujutsu-pinning.md)) |
| **M3** Auth + permissions | `LocalIdentityProvider` (Argon2id + HMAC) + `InMemoryPermissionIndex` + `FilesystemPermissionIndex` + `require_permission!` | ✅ Done |
| **M4** Cache layer stub | `ReadCache` trait + `InProcessCache` (`Arc<DashMap>`) + `CachedContentStore` wrapper warming on read / invalidating on write+undo | ✅ Done |
| **M5** FFI bridge | `BridgeServices<S,P,I,R>` + 5 token-gated FFI methods + `WorkspaceRegistry` + `PageSnapshot` / `WorkspaceSummary` wire types | ✅ Rust side done (TASK-011); Dart-side codegen + `flutter test` pending TASK-012 (blocked on M6 scaffolding `app/` + `sdk/liquid_sdk/`) |
| **M6.5** Minimal agent CLI (gates M6 per CLAUDE.md rule 6) | `liquid workspace create`, `page read/write/undo`, `auth provision-agent/token`, `audit list` — drives the MVP slice (`tests/cli/00_mvp_slice.bats`) plus focused subcommand coverage (`tests/cli/10_cli_subcommands.bats`) | ✅ Done (TASK-008) |
| **M6** Flutter shell skeleton | `RootShell` + `ExplorerPanel` (workspace switcher) + `PageArea` (toolbar) + `PageGrid` (12×12 + drag + resize) + placeholder `GridItem`; Riverpod state | ✅ Done (TASK-013 — widget tests 4/4; visual validation on a real display deferred) |
| **M8** Public Dart SDK | `LiquidComponent` + `InputSlot` / `OutputSlot` / `SlotSchema` / `SlotValue` + `AppManifest` / `ComponentManifest` / `Permission` + abstract `GridApi` / `VcsApi` / `PermissionApi` | ✅ Done (TASK-015 — typed API + 6 tests); concrete FFI-backed runtime APIs pending TASK-012 |
| **M9** Data binding broker (Rust + Dart) | `liquid-bindings::SlotBroker` trait + `InProcessSlotBroker` + `SlotWiring` / `BindingsDocument` (Rust side); FFI exposure + Dart `OutputSlot.emit` / `InputSlot.stream` + page-grid wiring UI pending | ✅ Rust side done (TASK-016a — 9 tests); Dart side TASK-012, wiring UI TASK-016b |
| **M10** Multi-instance tenant configuration | `TenantConfigSchema` declared in `AppManifest`; encrypted persistence + UI form generation + AES-256-GCM key derivation | Planned (TASK-017 — depends on M5 Dart side + M11 first-party apps) |
| **M7** Full agent CLI | Rest of [§12](IMPLEMENTATION_PLAN.md#12-agent-cli-specification): `workspace list/delete`, `page history`, `auth login/whoami`, `--as` impersonation. `app …` subset deferred to TASK-014 (needs M8 `AppManifest`). | ✅ Done (TASK-009 shipped; TASK-014 for `app …`) |
| Phases 2–4 | SDK, first-party apps, mobile, scale, ecosystem | See [`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md) |

The full milestone breakdown lives in [`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md);
the active task queue lives in [`TASKS.md`](TASKS.md).

## Why Liquid?

| Problem today | Liquid's answer |
|---|---|
| Every app reinvents user management, roles, and permissions | First-class user/permission layer shared across all apps |
| Agents editing production content can lose data | VCS-native — every change is versioned, reversible, attributable |
| "Cross-platform" tools treat mobile as second-class | Single Dart codebase targets Linux/Windows/macOS/iOS/Android via Flutter Impeller |
| Subscriptions required for core functionality | Fully self-hostable, no SaaS dependency |
| Components are siloed inside apps; data can't flow between them | Cross-app component data binding via typed slots |
| Agent access is coarse-grained or uncontrolled | Agents are principals with identity, roles, and fine-grained permissions; they operate via CLI |
| Frameworks aren't built for enterprise scale | Each workspace independently scales to 10 000+ users + agents and millions of files |

## Quickstart

> Liquid is pre-alpha. There is no end-user binary yet. The instructions below
> are for **contributors and early integrators** who want to build from source
> and run the test suite.

### Prerequisites

| Tool | Version | Install |
|---|---|---|
| Rust | 1.94.1 (pinned) | <https://rustup.rs> — `core/rust-toolchain.toml` selects the right version automatically |
| Flutter | stable channel | <https://flutter.dev/docs/get-started> *(only needed once the shell lands in M6)* |
| `just` | latest | `cargo install just` |
| `lefthook` | latest | `npm install -g @evilmartians/lefthook` |
| `bats` | latest | <https://bats-core.readthedocs.io/en/stable/installation.html> *(needed for CLI tests once M6.5 lands; `tests/cli/00_mvp_slice.bats` is the M6.5 acceptance gate)* |

### Build and test

```sh
git clone https://github.com/open-equilibrium/liquid.git
cd liquid

just install-hooks                        # wire git hooks via lefthook
cargo test --manifest-path core/Cargo.toml --workspace   # run the full Rust test suite
just check                                # full pre-push validation: lint + test
```

That's everything Phase 1 currently exercises. Flutter / Dart commands light up
as the relevant packages land — see [`CONTRIBUTING.md`](CONTRIBUTING.md) for the
full developer workflow and `just --list` for every available command.

## Documentation

| Audience | Where to look |
|---|---|
| **App developers** building on Liquid | `docs/sdk-guide/` *(populates in Phase 2)* |
| **Operators / self-hosters** | `docs/operations/` *(populates in Phase 3)* |
| **Contributors** to Liquid itself | [`CONTRIBUTING.md`](CONTRIBUTING.md) + [`DEVELOPER_INFO.md`](DEVELOPER_INFO.md) |
| **Architecture & design rationale** | [`DEVELOPER_INFO.md`](DEVELOPER_INFO.md), [`docs/adr/`](docs/adr/), [`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md) |
| **Auditors validating a Phase-1 milestone** | [`docs/manual-validation-m1-m3.md`](docs/manual-validation-m1-m3.md), [`docs/manual-validation-m4-m5.md`](docs/manual-validation-m4-m5.md), [`docs/manual-validation-m6.5.md`](docs/manual-validation-m6.5.md) |
| **AI agents working on this repo** | [`CLAUDE.md`](CLAUDE.md) — mandatory project workflow |

## Contributing

Contributions are welcome — bug reports, code, docs, design discussions all
help. Please:

1. Read [`CONTRIBUTING.md`](CONTRIBUTING.md) — covers setup, the TDD-first
   workflow, Conventional Commits, and PR expectations.
2. Read [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) — Liquid follows the
   Contributor Covenant 2.1.
3. Pick a task from [`TASKS.md`](TASKS.md), or open an issue describing what
   you'd like to work on before implementing.
4. Open a PR against `main`. Every PR runs the full Rust test suite, fmt,
   clippy, and (once those layers exist) Flutter analyze + tests + bats.

> **Note on cadence:** Liquid is a single-maintainer, spare-time project
> until it tags `v1.0.0`. PR review and issue triage happen as the
> maintainer's schedule allows — please don't read silence as rejection.
> The *Pre-1.0 obligations checklist* in
> [`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md) tracks what becomes
> a real commitment at v1.0.

## Security

Found a security issue? **Do not open a public issue.** Report it via GitHub's
private vulnerability reporting on this repository. Pre-1.0 there are no
SLA commitments; the policy at v1.0 is described in
[`SECURITY.md`](SECURITY.md).

## Community / contact

There is **no public contact channel and no chat / mailing list /
office-hours yet** — those open at v1.0 and are tracked under the
*Pre-1.0 obligations checklist* in
[`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md). Until then:

- **Questions / discussion:** open a GitHub Discussion (or an Issue
  with the `question` label).
- **Bugs / requests:** the issue templates in
  [`.github/ISSUE_TEMPLATE/`](.github/ISSUE_TEMPLATE/).
- **Security reports:** GitHub Security Advisories on this repository
  (see [`SECURITY.md`](SECURITY.md)).
- **Roadmap context:** [`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md).

## License

Liquid is licensed under the **Apache License, Version 2.0** — see
[`LICENSE`](LICENSE) and [`NOTICE`](NOTICE). Contributions are accepted under
the same license.
