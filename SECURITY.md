# Security Policy

> **Status:** Liquid is **pre-alpha** — a single-maintainer, spare-time
> project under active early development. **No security guarantees,
> response-time commitments, supported-version promises, or formal
> disclosure SLAs apply until the project tags its first stable
> release (`v1.0.0`).** This file describes how to *report* a security
> issue today and what the project *intends* to commit to once it
> reaches that release.

## Reporting a security issue

**Please do not open a public GitHub issue, pull request, or discussion
to report a security concern.** Public reports give attackers a window
of opportunity before a fix is available.

Instead, use **GitHub's private vulnerability reporting** on this
repository:

> 1. Go to <https://github.com/open-equilibrium/liquid/security/advisories>
> 2. Click **"Report a vulnerability"**
> 3. Fill in the form. GitHub routes the report privately to the
>    repository administrators and creates a secure draft advisory the
>    maintainer can collaborate in.

When reporting, please include as much of the following as you can:

- A description of the issue and the impact you believe it has
- A minimal reproducer (a few lines of code or a CLI invocation is ideal)
- The Liquid version, commit SHA, or release tag affected
- Your environment (OS, Rust version, Flutter version if relevant)
- Whether the issue is already public anywhere

You do **not** need to provide a fix to file a report.

## What you should expect today (pre-1.0)

- **No dedicated security contact alias** is published yet. GitHub's
  private vulnerability reporting (link above) is the only supported
  channel. The repository administrator account on the GitHub side is
  the reporting destination.
- **No response-time commitment.** This is a spare-time project and
  the maintainer cannot promise a specific acknowledgement, triage, or
  fix window. Reports are looked at as time permits, in priority order
  by severity. Reporters who want to share a request for response time
  in the report itself are welcome to.
- **No supported-version promise.** Only the current `main` branch
  receives any kind of attention. There are no tagged releases yet to
  patch.
- **No coordinated-disclosure timeline.** The maintainer will use
  reasonable judgement on when to publish a fix and an advisory; the
  reporter is welcome to discuss timing in the private advisory.
- **Good-faith reporting is appreciated.** The maintainer will not
  pursue legal action against reporters who follow this policy in
  good faith — but this is a statement of intent, not a contract or
  legal undertaking, until the project's governance and any
  bug-bounty / safe-harbor language is formalised at v1.0.

## What's planned at v1.0 (not in effect yet)

When the project tags `v1.0.0` it will adopt a formal security policy.
Until then this section is the maintainer's intent, not a promise:

- A real `security@…` contact alias on a project-owned domain
- Acknowledgement / triage / fix windows (CERT/CC-style, e.g.
  3 / 10 / 90 business days)
- A defined supported-version range (`v1.x` plus the previous minor)
- A documented coordinated-disclosure window
- Reporter credit in the published advisory and in `CHANGELOG.md`,
  unless the reporter requests anonymity
- A safe-harbor statement aligned with
  [disclose.io](https://disclose.io/)

The `IMPLEMENTATION_PLAN.md` *Pre-1.0 obligations checklist*
(`§ Pre-1.0 obligations`) tracks these as gating items for the first
release.

## Scope

In scope for this policy (when it becomes binding at v1.0):

- The Rust crates under `core/` (`liquid-core`, `liquid-vcs`,
  `liquid-auth`, `liquid-permissions`, `liquid-cache`, `liquid-bindings`,
  `liquid-sdk-bridge`, `liquid-cli`).
- The Flutter app under `app/` (when it lands in Phase 1).
- The public Dart SDK under `sdk/liquid_sdk/` (when it lands in Phase 2).
- The agent CLI (`liquid`, when it lands in Phase 1).
- The self-hosted package registry under `registry/` (when it lands in
  Phase 3).
- Any signed manifest or capability-token format defined by the project.

Out of scope (file an issue normally instead):

- Bugs that do not have security impact (functional defects, performance
  regressions, cosmetic issues).
- Vulnerabilities in third-party dependencies that are patched upstream
  but have not yet been pulled in — please report those upstream first
  and let this project know which version of Liquid will need an update.
- Issues in software that is not part of this repository (e.g. the
  reader's own Liquid app, their hosting environment, their browser).

## Hardening properties the code aims for

These are design properties the codebase is being built around. They
are **not** guarantees — pre-alpha code may regress against any of
them. Treat this list as the bar the project is *trying* to clear, not
a contract:

- Permission checks are intended to be the first line of every
  `liquid-sdk-bridge` FFI call (`require_permission!` macro).
- Passwords are hashed with **Argon2id** via the `argon2` crate; raw
  passwords are not persisted.
- Session tokens are **HMAC-SHA256** signed; tampered, expired,
  or wrong-key tokens fail closed with the same opaque
  `LiquidError::Forbidden` so the failure mode is not leaked.
- Workspace partitioning is enforced — every storage and permission
  call carries a `WorkspaceId`; there is no global namespace.
- The Rust workspace forbids `unsafe_code` and lints `unwrap()` /
  `expect()` outside `#[cfg(test)]` (`core/Cargo.toml`).
- Once signed manifests ship in Phase 2, the runtime will refuse to
  load unsigned or tampered packages.

See [`DEVELOPER_INFO.md`](DEVELOPER_INFO.md) → *Core Design Principles
→ Security* for the longer-form rationale.

---

If anything in this policy is unclear, please reach out via the GitHub
flow above before assuming. Clear communication is the most important
security control either party has.
