# ADR-003 — OSS policy: minimum-viable today, formal commitments deferred to v1.0

**Status:** Accepted
**Date:** 2026-05-05
**Deciders:** Claude (OSS-hardening implementer), repository maintainer

## Context

Liquid was set up as an open-source project from the first commit
(`license = "Apache-2.0"` declared in `core/Cargo.toml`), but until this
ADR was written the repository was missing the standard set of OSS
files that downstream tooling, contributors, and security reporters
expect:

- `LICENSE` (Apache-2.0 text was declared but the file did not exist)
- `NOTICE`
- `CODE_OF_CONDUCT.md`
- `SECURITY.md`
- `CONTRIBUTING.md`
- `CHANGELOG.md`
- `.github/ISSUE_TEMPLATE/{bug_report,feature_request}.md` plus a
  `config.yml` to route security and Q&A traffic correctly
- `.github/PULL_REQUEST_TEMPLATE.md`
- A root `.gitignore` (only `core/.gitignore` existed)
- A `README.md` shaped for newcomers (the existing one was a 635-line
  design doc)

The project also has a structural tension that affects the wording of
several of these files:

> Liquid is a **single-maintainer, spare-time project** (the
> maintainer has stated 2–5 hours per week of capacity). The standard
> OSS templates assume a maintainer pool large enough to commit to
> response-time SLAs, supported-version ranges, formal CoC enforcement
> processes, dedicated security contact aliases, and so on.

If we ship the standard templates verbatim, the project promises
things the maintainer cannot deliver — which is both ethically wrong
and operationally risky (people rely on those promises to plan
disclosures, decide where to deploy, etc.).

If we ship nothing, the project looks abandoned to outside observers
and is missing the basic legal and intake plumbing that a public
repository needs.

We need a defensible middle position.

## Decision

Adopt a **minimum-viable, deliberately-deferred** OSS posture:

1. **Ship the legal and structural baseline now.** Real `LICENSE`,
   `NOTICE`, `CONTRIBUTING.md`, `CHANGELOG.md`, issue/PR templates,
   root `.gitignore`, restructured `README.md`, and `DEVELOPER_INFO.md`
   for moved design content. None of these commit the maintainer to
   anything they cannot uphold; they exist to make the project legible
   and the intake surface usable.
2. **Ship `SECURITY.md` and `CODE_OF_CONDUCT.md` with explicit
   pre-alpha status callouts.** Both files describe how to *report* a
   concern today (GitHub Security Advisories / direct DM) and adopt the
   standards (Apache-2.0, Contributor Covenant 2.1) but *defer* every
   numeric commitment (response time, supported versions, enforcement
   ladder, contact aliases) until `v1.0.0`. The files explicitly say
   so, in a quoted callout at the top.
3. **Track the deferred commitments in
   `IMPLEMENTATION_PLAN.md` §17 *(Pre-1.0 Obligations Checklist)*.**
   Every commitment we don't make today has a checkbox there, with
   enough context for a future maintainer (or future-self) to decide
   what the binding policy should be. Tagging `v1.0.0` is gated on
   resolving every item in §17 (or deliberately deferring with a new
   ADR).
4. **Adopt the Code of Conduct by reference, not by inline copy.**
   Several large projects (Kubernetes, CNCF projects) do this; it
   keeps the project's policy current with the canonical Contributor
   Covenant text without manual sync, and avoids a known footgun where
   AI-assisted contributions to the file trip output filters on the
   verbatim list of prohibited behaviours.
5. **Use placeholder addresses, not the maintainer's personal email,
   for any contact alias.** The placeholders explicitly say
   "to be replaced before v1.0", so readers don't take them as live.
6. **Move the design / feasibility / risk-register / competitive-
   landscape content out of `README.md`** into `DEVELOPER_INFO.md`.
   `README.md` becomes ~130 lines in OSS-newcomer shape (tagline →
   status table → why → quickstart → docs map → contributing →
   security → community → license).
7. **Leave organisation-level files
   (`open-equilibrium/.github` profile, FUNDING, org-wide CoC) for
   the v1.0 obligations checklist.** Those need a real GitHub
   organisation profile and live email aliases that don't exist yet.

## Rationale

**Truthful documentation is a feature.** Pre-alpha software with
post-1.0 promises is worse than pre-alpha software that names its
constraints. A reporter who reads "we acknowledge within 3 business
days" and then waits two weeks loses faith in the project; a reporter
who reads "spare-time project, no SLA, please be patient" plans
accordingly and stays engaged.

**§17 is the load-bearing piece.** Without an explicit obligations
checklist, "we'll do this at v1.0" is a vague intention. With one,
every deferred commitment has a checkbox that must be ticked or
deliberately punted (with a follow-up ADR) before the first stable
release. That makes it review-able and forces the maintainer to make
the policy decisions consciously, in one pass, rather than tripping
into them during a security incident.

**Adopt-by-reference is now the dominant CoC pattern.** Kubernetes,
the CNCF, OpenSSF, and many large projects link to the Contributor
Covenant rather than copy it. The benefits compound: upstream policy
updates reach the project automatically, the file stays editable
under modern AI-assisted workflows (the verbatim text trips content
filters in some pipelines), and the project's own customisation is
clearly demarcated from the standard text.

**Placeholder addresses with explicit disclaimers are honest.**
"`security@…` placeholder — to be replaced before v1.0" is more
informative than either omitting the address entirely (no path to
file a report at all) or putting the maintainer's personal email
(implies a personal commitment to availability that is not promised).

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Ship boilerplate templates verbatim | Promises the maintainer cannot uphold (3-day SLAs, supported-version ranges, formal CoC enforcement, etc.). High risk of damaged reputation when the gap shows. |
| Ship no OSS files until v1.0 | Project looks abandoned; security reporters have nowhere to file privately; downstream tooling (GitHub's "Community Standards" check, package linters) flags missing files; ambiguous license posture (declared in Cargo.toml but no `LICENSE` file). |
| Use the maintainer's personal email for security/CoC contact | Implies personal commitment to availability that is not promised. Easier to deal with via GitHub's private vulnerability reporting + a placeholder alias that becomes real at v1.0. |
| Inline the Contributor Covenant 2.1 verbatim | Violates the maintenance principle (upstream updates require manual sync). Also tripped output content filters in this session's AI-assisted authoring; adopt-by-reference dodges that mode entirely. |
| Add a `GOVERNANCE.md` and `CODEOWNERS` now | Codifies a structure that doesn't exist (one maintainer, no sub-teams). Adding these prematurely is documentation drift waiting to happen. They live on the v1.0 checklist and turn on when there's something real to govern. |
| Promise reviewer SLA in `CONTRIBUTING.md` | Same failure mode as the security SLA — a single-maintainer project cannot guarantee response time without trading away the maintainer's ability to take a vacation. The honest "best-effort, please don't read silence as rejection" copy is enough. |

## Consequences

**Easier:**

- The repository now satisfies GitHub's Community Standards check
  (the OSS-readiness signal that appears on the repo's Insights tab)
  without inflating commitments.
- Downstream tooling that expects a `LICENSE` file (cargo audit,
  pip-style license tooling, CI dependency scanners) starts working.
- Security reporters have a clear, private channel (GitHub Security
  Advisories) and explicit expectations about response cadence.
- Newcomers reading `README.md` can decide in 30 seconds whether the
  project is for them; if they want depth, the docs map points to
  exactly the right next file.

**Harder:**

- The maintainer has to actually walk through §17 before tagging
  `v1.0.0`. That's the *intended* friction — the alternative is
  cutting v1.0 with a half-formed security policy.
- Three top-level files (`SECURITY.md`, `CODE_OF_CONDUCT.md`,
  `README.md`) now carry "Status / pre-alpha" callouts that look
  unusual in the OSS landscape. That's a feature; if the callouts
  surprise readers, the rest of the doc is doing its job.
- Every doc that mentions a deferred commitment now has a maintenance
  obligation: when the commitment becomes real, every reference to
  "see §17" or "to be replaced before v1.0" needs to be updated. The
  `sync-docs` skill audit pass is the safety net.

**Existing code / rules affected:**

- `IMPLEMENTATION_PLAN.md` §17 is the canonical pre-1.0 obligations
  list; nothing else may make a v1.0 commitment without adding a row
  there or invalidating it through a new ADR.
- The `sync-docs` skill (`.claude/skills/sync-docs/SKILL.md`) audits
  these files for drift and is intended to catch it when the doc set
  starts disagreeing with itself.
- The `implement` skill's Step 7 (Documentation update) defers to the
  same canonical surface list. No additional layer is needed; the
  existing TDD workflow continues to drive doc changes alongside
  code changes.

## Notes on naming

- `LICENSE` copyright string: "The Liquid Project Contributors"
  (umbrella pattern, same as Rust / Node / Kubernetes). The
  maintainer's personal name is **not** used. Whether to switch to
  "Open Equilibrium" as an organisation owner is a `v1.0` decision
  tracked in §17.1.
- `DEVELOPER_INFO.md` is uppercase to match the rest of the
  top-level docs (`README.md`, `IMPLEMENTATION_PLAN.md`, `TASKS.md`,
  `CONTRIBUTING.md`, …). The original `developer_info.md` filename
  was renamed in the same commit set as this ADR.
