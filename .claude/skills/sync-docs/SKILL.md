---
name: sync-docs
description: Audit Liquid's documentation set for drift against the current state of the code. Use after any implementation task that changes a public Rust trait, FFI surface, SDK API, CLI command, data model, or workspace layout — and before opening a PR. Reports drift; does not auto-edit unless the user asks for fixes.
---

# Sync Docs

Liquid's documentation is **load-bearing**: AI agents and human contributors
both rely on it as the source of truth for what the project ships. The
`implement` skill already has a Step 7 ("Documentation update") that prompts
the author to update docs alongside code; this skill is the auditor that
runs *after* that step (or before a PR) and flags anything that fell
through the cracks.

## When to invoke

- After completing an implementation task (the `implement` skill's
  Step 7 calls this skill out as the recommended verification).
- Before opening a PR that changes anything user-visible or
  agent-callable.
- Whenever the user asks "are the docs in sync?" or "what changed in
  the public API since the last release?".

Skip if the diff is purely an internal refactor that touches no public
API, no CLI grammar, no schema, and no design decision.

## Inputs

- The current `git diff main...HEAD` (or `git diff` for uncommitted
  changes).
- The set of "doc surfaces" listed below.

## Doc surfaces to check

Run through each surface and decide whether the diff requires an update.

| If the diff changes… | Then update… |
|---|---|
| A public Rust trait or function in any `core/liquid-*` crate | `IMPLEMENTATION_PLAN.md` §4 (interfaces) and/or §9 (crate reference) |
| The FFI surface in `liquid-sdk-bridge` | `IMPLEMENTATION_PLAN.md` §5.5 + §9 |
| The `liquid` CLI grammar (any clap subcommand or flag) | `IMPLEMENTATION_PLAN.md` §12 |
| A public SDK API in `sdk/liquid_sdk/lib/` | `IMPLEMENTATION_PLAN.md` §11 + `docs/sdk-guide/` |
| A built-in role's permission set (`BuiltInRole::permits`) | `IMPLEMENTATION_PLAN.md` §9 *(Built-in roles table)* |
| Token format, password hash, or any auth wire format | `IMPLEMENTATION_PLAN.md` §4.5 + §9 (`liquid-auth`) + `SECURITY.md` |
| Storage layout on disk (`FilesystemContentStore` etc.) | `IMPLEMENTATION_PLAN.md` §9 *(`liquid-vcs` layout)* + the relevant ADR |
| A new `core/liquid-*` crate | `IMPLEMENTATION_PLAN.md` §2 (repo layout) + §9 (crate reference) + the workspace `Cargo.toml` member list |
| A milestone moves from Planned → Done | `README.md` *(status table)* + `TASKS.md` *(move task to Done section)* + `CHANGELOG.md` *(under `## [Unreleased]`)* |
| A design decision contradicting or extending an ADR | New ADR file in `docs/adr/NNN-title.md` using `docs/adr/TEMPLATE.md` |
| User-visible behaviour | `CHANGELOG.md` (`## [Unreleased]` section, Conventional-Commits-style entry) |
| The `.claude/` config (skills, agents, hooks, rules, settings) | `CLAUDE.md` "Claude Code Tooling" section |
| Project Absolute Rules | `CLAUDE.md` rule list + `CONTRIBUTING.md` "Project Absolute Rules" + the relevant ADR |

## Cross-reference checks

Beyond per-surface drift, sanity-check that the doc set agrees with itself
and with the code:

1. **Crate inventory.** Every member listed in `core/Cargo.toml`
   `[workspace]` appears in `IMPLEMENTATION_PLAN.md` §2 (layout), §9
   (reference), `README.md` quickstart-level mentions where relevant,
   and the `CONTRIBUTING.md` *"Layout you'll touch most"* table.
2. **Status table.** Every milestone marked ✅ in `README.md`'s status
   table has a corresponding `## [Unreleased]` or versioned entry in
   `CHANGELOG.md` and a Done task in `TASKS.md`.
3. **TASK-NNN references.** Every `TASK-NNN` referenced in
   `IMPLEMENTATION_PLAN.md`, `CHANGELOG.md`, or any ADR resolves to an
   entry in `TASKS.md`.
4. **ADR coverage.** Every ADR in `docs/adr/` is referenced from
   `IMPLEMENTATION_PLAN.md` (either §15's inline ADR list or in the
   relevant milestone / crate-reference section).
5. **§4 trait shapes match `lib.rs`.** For each trait in
   `IMPLEMENTATION_PLAN.md` §4, the documented signature matches the
   actual `pub trait` declaration in the crate. Common drift: error
   types (parallel `XxxError` vs. `LiquidError`), parameter additions,
   method removals.
6. **CLI grammar.** Every command listed in `IMPLEMENTATION_PLAN.md`
   §12 has a matching `clap` subcommand in `liquid-cli` (and vice
   versa). Once `tests/cli/` exists, every documented command has a
   bats test covering the happy path.
7. **CHANGELOG entries pre-1.0.** All pending behaviour changes since
   the last release tag live under `## [Unreleased]`; nothing is in a
   numbered version that hasn't been tagged.
8. **License headers.** Apache-2.0 is declared in `core/Cargo.toml`
   workspace metadata; `LICENSE` and `NOTICE` exist at repo root and
   are consistent.

## Output

Return a JSON object describing what drifted and what to do about it. Do
not auto-edit unless the user explicitly asks ("fix the drift", "apply
the suggestions", "update the docs"). Citing file paths and section
numbers makes the next agent's job easier.

```json
{
  "summary": "One sentence: how aligned is the doc set right now?",
  "critical_drift": [
    {
      "where": "IMPLEMENTATION_PLAN.md §4.2",
      "why": "PermissionIndex trait still documents `grant(...)` but the trait no longer has it.",
      "fix": "Remove the `grant` method from the §4.2 code block; cite ADR-002.",
      "evidence": "core/liquid-permissions/src/index.rs:33"
    }
  ],
  "warnings": [],
  "suggestions": [],
  "cross_reference_failures": [],
  "next_action": "Either ask the user to apply the listed fixes, or — if asked — apply them in a follow-up `docs(...)` commit on the same branch."
}
```

## Rules

- Cite specific file paths and section numbers; never wave at "the docs".
- Distinguish **critical drift** (docs claim something that contradicts
  the code) from **warnings** (docs are stale but not wrong) and
  **suggestions** (places the docs could be clearer).
- Do not edit files unless the user explicitly asks for fixes.
- If the diff is large, delegate the audit of any single doc to the
  `code-reviewer` subagent and aggregate the findings.
- One follow-up commit is enough for the doc fixes — they should ride
  in a single `docs(...)` commit, not be smeared across the
  implementation history.
