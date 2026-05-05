# Security Policy

We take the security of the Liquid project — and of every system that
depends on it — seriously. Thank you for helping us keep it safe.

## Reporting a security issue

**Please do not open a public GitHub issue, pull request, or discussion
to report a security concern.** Public reports give attackers a window
of opportunity before a fix is available.

Instead, use **GitHub's private vulnerability reporting** on this
repository:

> 1. Go to <https://github.com/open-equilibrium/liquid/security/advisories>
> 2. Click **"Report a vulnerability"**
> 3. Fill in the form. GitHub routes the report privately to the
>    maintainers and creates a secure draft advisory we can collaborate
>    in.

If GitHub's private reporting is not available to you, contact the
maintainers directly at:

> **security@liquid-project.org** *(placeholder — to be replaced with
> the active maintainer alias before general availability; until then,
> please use the GitHub flow above).*

When reporting, please include as much of the following as you can:

- A description of the issue and the impact you believe it has
- A minimal reproducer (a few lines of code or a CLI invocation is ideal)
- The Liquid version, commit SHA, or release tag affected
- Your environment (OS, Rust version, Flutter version if relevant)
- Whether the issue is already public anywhere

You do **not** need to provide a fix to file a report. We're happy to
work the fix collaboratively in a private advisory.

## Our commitment

When we receive a report, we will:

1. **Acknowledge** receipt within **3 business days**.
2. **Triage** and confirm or reject the report within **10 business days**.
3. **Communicate** a remediation timeline as soon as one is realistic to
   estimate. For confirmed issues we aim to ship a fix within **90 days**
   of the initial report (the disclosure window the
   [CERT/CC](https://www.kb.cert.org/vuls/) treats as standard).
4. **Coordinate disclosure** with the reporter — we will not publicly
   disclose the issue until either the fix has shipped or the 90-day
   window has elapsed, whichever is sooner.
5. **Credit** the reporter in the security advisory unless they ask to
   remain anonymous.

If a report turns out to be a duplicate, already-public, or
out-of-scope, we will say so promptly.

## Supported versions

Liquid is currently in **pre-alpha** (Phase 1 in progress). Until we
reach a 1.0 release, **only the latest commit on `main` is supported**
for security fixes. Once we cut tagged releases, this section will be
updated to indicate which version range receives patches.

| Version | Supported |
|---|---|
| `main` (latest) | ✅ |
| Pre-1.0 tagged releases | Best-effort, latest minor only |
| 1.x (when released) | ✅ until 2.0 ships, then `1.x − 1` series for 12 months |

## Scope

In scope for this policy:

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
  but we have not yet pulled in — please report those upstream first
  and let us know which version of Liquid will need an update.
- Issues in software that is not part of this repository (e.g. the
  reader's own Liquid app, their hosting environment, their browser).

## Hardening defaults you can rely on

Several security properties are part of Liquid's design from day one
(see [`developer_info.md`](developer_info.md) → *Core Design Principles
→ Security*):

- Permission checks are the first line of every `liquid-sdk-bridge`
  FFI call (`require_permission!`).
- Passwords are hashed with **Argon2id** via the `argon2` crate; raw
  passwords are never persisted.
- Session tokens are **HMAC-SHA256** signed; tampered, expired,
  or wrong-key tokens all fail closed with the same opaque
  `Forbidden` error so we don't leak which mode failed.
- Workspace partitioning is non-negotiable — every storage and
  permission call carries a `WorkspaceId`; there is no global
  namespace.
- The Rust workspace forbids `unsafe_code` and lints `unwrap()` /
  `expect()` outside `#[cfg(test)]` (`core/Cargo.toml`).
- Once signed manifests ship in Phase 2, the runtime will refuse to
  load unsigned or tampered packages.

## Acknowledging reporters

We maintain a list of people who have responsibly reported security
issues to the project. Reporters who would like to be credited are
acknowledged in the security advisory we publish for their report and
in the corresponding [`CHANGELOG.md`](CHANGELOG.md) entry.

---

If anything in this policy is unclear, please reach out via the
contacts above before assuming. Clear communication is the most
important security control either of us has.
