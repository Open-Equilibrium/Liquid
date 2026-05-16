# Branch Protection — `main`

This document captures the GitHub branch-protection rule that the
maintainer should enable for `main`. Until the rule is in place,
nothing prevents a direct push to `main` (the local `pre-push`
branch-name gate in `scripts/check-branch-name.sh` catches the case
locally, but a contributor could disable lefthook or use a different
client). Branch protection is the server-side enforcement that makes
the policy real.

## Required status checks on `main`

Enable **"Require status checks to pass before merging"** with the
following checks marked as required. The names match the `name:`
field on each job in `.github/workflows/`:

| Workflow | Job(s) marked required |
|---|---|
| `CI` (`.github/workflows/ci.yml`) | `Rust (ubuntu-latest)` |
| `CI` (`.github/workflows/ci.yml`) | `CLI bats tests` |
| `Audit` (`.github/workflows/audit.yml`) | `cargo audit` |
| `Audit` (`.github/workflows/audit.yml`) | `cargo deny` |
| `ai-check` (`.github/workflows/ai-check.yml`) | `ai-check` |
| `sync-docs` (`.github/workflows/sync-docs.yml`) | `sync-docs` |

Notes:

- Only the Ubuntu runner of the Rust matrix is marked required —
  Windows and macOS still run and gate the matrix's overall
  `fail-fast: false` outcome, but flagging the slowest runners as
  required has historically blocked merges for unrelated runner
  outages. Keep them visible but advisory.
- The Dart SDK and Flutter app jobs are **not** marked required
  while `sdk/liquid_sdk/` and `app/` are still being scaffolded —
  the detect-layer job currently skips them. Once Phase-1 M6 lands
  and those layers exist, add them to the required set in the same
  PR that ships the layer.
- `cargo audit` and `cargo deny` together cover advisories +
  licenses + bans + sources. Both jobs must be green.

## Additional protections

Beyond the status-check requirement, set:

| Setting | Value | Rationale |
|---|---|---|
| **Require a pull request before merging** | On | No direct pushes to `main`. The local `pre-push` gate already blocks `main` for human / agent pushes; this is the server-side backstop. |
| **Require approvals** | 1 | Single-maintainer project today; bump when a second reviewer is on-boarded. |
| **Dismiss stale pull request approvals when new commits are pushed** | On | An approval given against commit A must not auto-apply to commit B. |
| **Require linear history** | On | We squash-merge or fast-forward; no merge commits on `main`. Matches the strategy documented in `CONTRIBUTING.md`. |
| **Require signed commits** | Off (for now) | Sign-off is encouraged but enforcement is deferred until the maintainer-keys policy in `IMPLEMENTATION_PLAN.md` §17 is decided. |
| **Restrict who can push to matching branches** | Maintainers only | Belt-and-braces with "Require PR before merging". |
| **Allow force pushes** | Off | Bare `--force` is also blocked client-side via `.claude/settings.json`. |
| **Allow deletions** | Off | `main` is permanent. |

## How to enable (maintainer task)

1. Open the repo on GitHub → **Settings** → **Branches** →
   **Branch protection rules** → **Add branch ruleset**.
2. Set **Target branches** = `main`.
3. Apply every row in the tables above.
4. Save.
5. Verify by opening a trivial PR — the merge button should be
   disabled until every required check is green, and direct pushes
   to `main` should be rejected with `protected branch hook declined`.

## Why this is documented (and not auto-applied)

GitHub branch protection is configured via the repo settings UI or
the REST API and requires admin-level credentials. CI workflows
cannot grant themselves protection rules without elevated
permissions, and committing API tokens with admin scope to a public
repo is a non-starter. The right enforcement point is therefore the
maintainer, with this document as the checklist.

## Related

- Local enforcement: `scripts/check-branch-name.sh` +
  `lefthook.yml` `pre-push` `branch-name` step.
- Policy source: `CLAUDE.md` "Mandatory Development Workflow" and
  the per-session goal block.
- CI workflows: `.github/workflows/{ci,audit,ai-check,sync-docs}.yml`.
