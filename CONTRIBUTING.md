# Contributing to Liquid

Thank you for considering a contribution. Liquid is in early development —
every well-scoped patch, bug report, and design discussion meaningfully
shapes the project.

> **Heads up:** Liquid is a **single-maintainer, spare-time project**
> until it tags `v1.0.0`. Reviews, replies, and merges happen as the
> maintainer's schedule allows; please don't read silence as rejection.
> The `IMPLEMENTATION_PLAN.md` *Pre-1.0 obligations checklist* tracks
> what becomes a real commitment at v1.0 (response-time targets,
> contact aliases, etc.). Until then, every cadence number in this
> file is best-effort, not a promise.

This document covers everything you need to know to make a good first
contribution. For project context (vision, architecture, design rationale)
see [`DEVELOPER_INFO.md`](DEVELOPER_INFO.md). For day-to-day project rules
that apply equally to humans and AI agents, see [`CLAUDE.md`](CLAUDE.md).

## Code of Conduct

Liquid follows the **Contributor Covenant 2.1** — see
[`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md). Reports go through the channel
listed there.

## How to contribute

### Report a bug or request a feature

Use the templates in [`.github/ISSUE_TEMPLATE/`](.github/ISSUE_TEMPLATE/).
Please include the smallest reproducer you can, the version / commit you
hit it on, and (for bugs) the actual vs. expected behaviour.

### Pick a task

The active queue lives in [`TASKS.md`](TASKS.md). If a task interests you:

1. Comment on the corresponding issue (or open one referencing the
   task ID, e.g. *"TASK-007: working on this"*) so we don't duplicate work.
2. Read the milestone in [`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md)
   that the task belongs to.
3. Follow the workflow below.

### Propose a larger change

For anything that changes a public Rust trait, an FFI surface, the SDK API,
the data model, or the agent CLI grammar: open a GitHub Discussion or a
draft PR with a written rationale **before** investing days of work. The
maintainer would much rather discuss design upfront than ask you to throw
away a fully-written PR because the boundary was wrong — but please
don't expect immediate engagement; allow several days.

For decisions that contradict or extend an existing ADR, add a new ADR in
[`docs/adr/`](docs/adr/) using
[`docs/adr/TEMPLATE.md`](docs/adr/TEMPLATE.md).

## Development environment

### Prerequisites

| Tool | Version | Install |
|---|---|---|
| Rust | `1.94.1` (pinned via `core/rust-toolchain.toml`) | <https://rustup.rs> |
| Flutter | stable channel | <https://flutter.dev/docs/get-started> *(only needed once `app/` and `sdk/liquid_sdk/` exist)* |
| `just` | latest | `cargo install just` |
| `lefthook` | latest | `npm install -g @evilmartians/lefthook` |
| Docker | 24+ | <https://docs.docker.com/get-docker/> *(only needed for `just services-up` in Phase 3)* |
| `bats` | latest | <https://bats-core.readthedocs.io/en/stable/installation.html> *(only needed once `tests/cli/` exists)* |
| `cargo-deny` | latest | `cargo install --locked cargo-deny` *(needed for `just deny-check` / `just check`; lefthook's `pre-push` and CI's `audit.yml` run the same gate)* |
| `cargo-tarpaulin` | `^0.31` | `cargo install --locked cargo-tarpaulin --version ^0.31` *(needed for `just coverage-check` / `just check`; matches the version pinned in `.github/workflows/ci.yml`'s Rust job)* |

### First-time setup

```sh
git clone https://github.com/open-equilibrium/liquid.git && cd liquid
just install-hooks                      # wires git hooks via lefthook
cargo test --manifest-path core/Cargo.toml --workspace   # sanity-check
```

### Daily commands

```sh
just test          # all tests (Rust now; Flutter + SDK + CLI bats as they land)
just lint          # all linters (clippy + dart analyze + dart format)
just fmt           # auto-fix all formatting
just check         # full pre-push validation: lint → test → deny-check (matches CI)
just run           # flutter run -d linux  (or macos / windows)  — when M6 lands
just cli -- --help # run the liquid CLI                          — when M7 lands
just services-up   # start Redis / Redpanda                       — Phase 3+
```

`just --list` shows every recipe at any point.

### Layout you'll touch most

| Path | Purpose |
|---|---|
| `core/` | Rust Cargo workspace — all business logic |
| `core/liquid-core/` | Shared primitives (`WorkspaceId`, `PrincipalId`, `LiquidError`, …) |
| `core/liquid-vcs/` | `ContentStore` trait + `InMemory` and `Filesystem` implementations |
| `core/liquid-auth/` | `IdentityProvider` + `LocalIdentityProvider` (Argon2id + HMAC) |
| `core/liquid-permissions/` | `PermissionIndex` + `BuiltInRole` matrix + `require_permission!` macro |
| `app/` | Flutter app (rendering and input only — no business logic) |
| `sdk/liquid_sdk/` | Public Dart package for app developers |
| `tests/cli/` | bats tests for the `liquid` CLI |
| `docs/adr/` | Architecture Decision Records |
| `docs/sdk-guide/` | Developer-facing SDK guide |
| `.github/workflows/` | CI pipeline |
| `.claude/` | Repo-local AI agent config (skills, subagents, hooks, rules) |

## Workflow expectations

### TDD-first

Liquid is a TDD project — *every* behaviour change starts with a failing
test. The full red→green→cleanup loop is encoded in
[`.claude/skills/implement/SKILL.md`](.claude/skills/implement/SKILL.md);
the short version:

1. **Red** — write a failing test in the layer you're changing
   (`#[test]` in Rust, widget test in Flutter, bats test for CLI).
2. **Green** — write the minimum code that makes the test pass. No
   speculative abstractions, no scope creep.
3. **CLI before UI** — features that store/read/mutate data must be
   exercisable from the `liquid` CLI before any UI work begins.
4. **E2E** — for UI changes, add or update an integration test under
   `app/integration_test/`.
5. **Review pass** — `cargo fmt --check`, `cargo clippy --all-targets
   -- -D warnings`, `dart format`, `flutter analyze`, `bats tests/cli/`,
   self-review the diff.

### Project Absolute Rules (from `CLAUDE.md`)

These are non-negotiable for every PR. CI enforces most of them; reviewers
enforce the rest:

1. **No `unwrap()` / `expect()` outside `#[cfg(test)]`** — every error
   propagates via `Result`.
2. **No platform imports in app packages** — no `dart:io`, no Flutter
   plugins, no platform channels in `apps/` or `sdk/`. The
   `no_platform_imports` lint catches violations.
3. **No direct Dart references between components** — cross-component
   communication goes through `SlotBroker`. The
   `no_cross_component_reference` lint catches violations.
4. **Permission check is always first** — every `liquid-sdk-bridge`
   FFI function calls `require_permission!` before any other logic.
5. **Every storage call takes a `WorkspaceId`** — there is no global
   namespace.
6. **CLI before UI** — if the data path isn't proven via CLI, the UI
   isn't started.
7. **Failing test before implementation** — TDD is not optional.

### Conventional Commits

Format: `<type>(<scope>): <summary>`

| Type | When |
|---|---|
| `feat` | New feature (user-visible behaviour change) |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `refactor` | No behaviour change, internal cleanup |
| `test` | Tests only |
| `chore` | Tooling, dependencies, CI |
| `perf` | Performance improvement |

Common scopes: `core`, `vcs`, `auth`, `permissions`, `cache`, `bindings`,
`bridge`, `cli`, `app`, `sdk`, `registry`, `ci`, `deps`.

Examples:

```
feat(vcs): implement JujutsuContentStore read and write
fix(permissions): prevent role escalation in assign_role
test(cli): add bats tests for liquid page write command
docs(sdk): document Platform Abstraction Contract
```

The release tooling (`cargo-release`) generates changelogs from commit
history, so well-formed commits are load-bearing.

### Branching and PRs

- Branch from `main`. Use a descriptive branch name (e.g.
  `feat/m4-cache-stub`, `fix/permissions-scope-leak`).
- **One logical change per PR.** Smaller PRs ship faster and review better.
- Reference the relevant `TASK-NNN` and milestone in the PR body.
- Fill in the PR template — it surfaces the test plan and the project
  Absolute Rules checklist.
- Push to your branch; open a PR against `main`. Wait for CI green before
  asking for review.
- Merge strategy: **fast-forward** when possible, **squash-merge** for
  PRs with noisy fixup commits. We avoid merge commits except when
  long-running branches diverge significantly from `main`.

### Documentation as part of the change

Documentation updates ship in the same PR as the behaviour change, never
as a follow-up:

- New CLI command → update [`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md) §12
- New SDK API → update [`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md) §11 + `docs/sdk-guide/`
- Trait signature change → update [`IMPLEMENTATION_PLAN.md`](IMPLEMENTATION_PLAN.md) §4 / §9
- User-visible behaviour → update [`CHANGELOG.md`](CHANGELOG.md) under
  the `## [Unreleased]` heading
- Design decision contradicting/extending an existing one → new ADR in
  [`docs/adr/`](docs/adr/)

The CHANGELOG-discipline `commit-msg` hook
(`.lefthook/commit-msg/check-changelog.sh`) enforces this for the
behaviour-change types: any `feat(*)` / `fix(*)` / `refactor(*)` /
`perf(*)` / `chore(<non-tooling-scope>)` commit must either modify
`CHANGELOG.md` in the same commit or carry a `[no-changelog]`
trailer with a one-line justification. `docs(*)`, `test(*)`, and
`chore(ci|claude|deps|ai|gh|tooling)` are exempt. Covered by 14
bats cases in `tests/cli/04_changelog_gate.bats`.

For projects working with AI agents, the
[`.claude/skills/sync-docs/SKILL.md`](.claude/skills/sync-docs/SKILL.md)
skill audits doc drift before you commit.

## Reviewer expectations

Reviewers should look for:

- **Correctness:** does the test actually exercise the new behaviour?
- **Project rules:** any of the seven Absolute Rules violated?
- **Scope:** is the PR doing one thing? Drive-by refactors should be
  separate PRs.
- **Bloat:** any abstraction serving exactly one callsite? Any
  duplicated logic from an existing crate?
- **Stability:** any `unwrap()` / `expect()` outside `#[cfg(test)]`?
  Any input reaching storage without validation?
- **Performance:** any hot path that could be O(1) where O(log n) or
  O(n) was used?
- **Docs:** are user-visible / API-visible changes documented?

The [`code-reviewer` subagent](.claude/agents/code-reviewer.md) runs the
same checklist and is recommended for non-trivial diffs.

## License

By contributing, you agree that your contributions will be licensed under
the [Apache License 2.0](LICENSE), the same license as the project.
