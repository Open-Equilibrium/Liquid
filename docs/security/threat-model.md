# Liquid — Threat Model (pre-alpha)

> **Status / pre-alpha.** This document describes Liquid's current
> security posture as of Phase 1. It is **concrete about what exists**
> (auth, permissions, storage layout, FFI surface) and **explicit about
> what does not exist yet** (network surface, multi-host, signed
> manifests, hardened secrets storage). It will evolve milestone-by-
> milestone; before `v1.0.0` it converts into a binding policy
> alongside `SECURITY.md` per `IMPLEMENTATION_PLAN.md` §17.3.

## 1. Scope

This document covers the Liquid runtime running on a single workstation
(Linux / Windows / macOS) in Phase 1. Mobile (Phase 3), the package
registry (Phase 3+), and multi-host scaling (Phase 4) are explicitly
out of scope for now — each will get its own threat-model section
when the relevant milestone lands.

In scope:

- The eight Rust crates under `core/` (`liquid-core`, `liquid-vcs`,
  `liquid-auth`, `liquid-permissions`, `liquid-cache`,
  `liquid-bindings`, `liquid-sdk-bridge`, `liquid-cli`).
- The on-disk layout of a Liquid workspace
  (`FilesystemContentStore` + `LocalIdentityProvider` +
  `FilesystemPermissionIndex`).
- The agent-facing CLI surface (`liquid …`).
- The FFI bridge (`liquid-sdk-bridge` + the Dart shell calling into it).

Out of scope (today):

- Network listeners (none exist; Phase 3 adds Redis/Redpanda).
- Code signing / supply-chain attestation for app manifests
  (Phase 2; the Phase-1 stub fails open with a warning).
- Cryptographic key management at rest (no HSM, no OS keyring
  integration yet — the HMAC signing key is whatever the operator
  provides).
- Side-channel attacks against Argon2id / HMAC. We use the upstream
  `argon2` and `hmac` crates; we do not roll our own crypto.

## 2. Principals and trust boundaries

Liquid models three types of principal — all carry a `PrincipalId`
(`u:<uuid>` for users, `a:<uuid>` for agents):

| Principal | How it authenticates | Trust level |
|---|---|---|
| **User** | Argon2id-hashed password against `<root>/users.toml`, exchanged for a session token. | Operates the runtime; usually the workstation owner. |
| **Agent** | Provisioned by an authorising user (CLI: `liquid auth provision-agent`). Holds a session token; no password. | Constrained by the role assigned at provisioning; the user is liable for the agent's actions. |
| **Process (Rust core)** | Implicit — it runs as the OS user that launched it. | Highest trust within Liquid: holds the HMAC signing key in memory. |

Trust boundaries:

```
┌─────────────────────────────────────────────────────────────────────┐
│  OS / filesystem                                                    │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  Liquid process (Rust)                                        │  │
│  │  ┌──────────────────┐   ┌─────────────────────────────────┐   │  │
│  │  │ liquid-sdk-bridge│   │ liquid-cli                      │   │  │
│  │  │ (FFI surface)    │   │ (clap-driven subcommands)       │   │  │
│  │  └────────┬─────────┘   └────────┬────────────────────────┘   │  │
│  │           │                      │   <-- every callsite       │  │
│  │           ▼                      ▼       runs require_perm    │  │
│  │  ┌───────────────────────────────────────────────────────┐    │  │
│  │  │ liquid-permissions  ◄──  liquid-auth                  │    │  │
│  │  │   (RBAC check)           (token verify; Argon2id)     │    │  │
│  │  └───────────────────────────────────────────────────────┘    │  │
│  │                              │                                │  │
│  │                              ▼                                │  │
│  │  ┌───────────────────────────────────────────────────────┐    │  │
│  │  │ liquid-vcs::FilesystemContentStore  +  op_log.jsonl   │    │  │
│  │  └───────────────────────────────────────────────────────┘    │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  Flutter shell (Dart) ── FFI ──▶ liquid-sdk-bridge                  │
│                                  (UI is a thin client; no logic)    │
└─────────────────────────────────────────────────────────────────────┘
```

The OS user owning the workspace root has *complete* control over the
contents. Liquid does not encrypt data at rest, and does not defend
against an attacker who already has filesystem write access to
`<root>/workspaces/<id>/`.

## 3. Token format and lifecycle

Session tokens — issued by `LocalIdentityProvider` (`liquid-auth`):

```
<principal> . <expires_unix> . <hmac_hex>
```

- `principal` is `u:<uuid>` (user) or `a:<uuid>` (agent).
- `expires_unix` is a UTC seconds-since-epoch integer; default lifetime
  is 24 hours for users, 7 days for agents (ADR-002).
- `hmac_hex` is `HMAC-SHA256(signing_key, "<principal>.<expires_unix>")`,
  lowercase hex.

Verification (`LocalIdentityProvider::verify_token`):

1. Split on `.` into three fields; reject if not exactly three.
2. Recompute the HMAC and compare in constant time (`subtle::ConstantTimeEq`).
3. Check `expires_unix > now()`; reject if expired.
4. Confirm the principal exists in `users.toml` or `agents.toml`.

**All failure modes collapse to `LiquidError::Forbidden`.** The CLI never
distinguishes "wrong signature" from "expired" from "unknown principal" —
that's by design (ADR-002).

### Threats

| Threat | Mitigation |
|---|---|
| Token forgery without the signing key | HMAC-SHA256 + constant-time compare. Forging needs the key. |
| Replay after expiry | Embedded `expires_unix` checked on every verify; tokens are stateless so no revocation list — short lifetimes are the lever. |
| Replay before expiry | **Not mitigated.** A stolen unexpired token is fully usable until it expires. Mitigation roadmap: per-session token revocation list (Phase 3) and OAuth/OIDC for users (Phase 3). |
| Signing-key compromise | **Catastrophic** — every outstanding token becomes forgeable. Mitigation: store the key in the OS keyring (Phase 3 obligation in §17.5 of the plan); pre-1.0 the key lives in whatever file the operator provides. |
| Timing-side-channel on HMAC verify | `subtle` crate's constant-time compare. |
| Argon2id parameter regression | Parameters pinned in `liquid-auth::HASH_PARAMS`; any change is a behaviour-breaking bump and gets an ADR. |

## 4. Workspace isolation

A workspace is the unit of permission and storage isolation. Every
storage call carries a `WorkspaceId` (CLAUDE.md Absolute Rule 5). On
disk:

```
<root>/workspaces/<workspace_id>/
    files/<store_path>      ← content addressed by hash
    op_log.jsonl            ← per-write operation log
    permissions.toml        ← Binding list for FilesystemPermissionIndex
```

`StorePath` (in `liquid-core`) is the only way to address a file inside
a workspace and is validated on construction:

- Rejects absolute paths (`/etc/passwd`).
- Rejects path-traversal segments (`..`).
- Rejects empty segments and embedded NUL.
- UTF-8 only.

**Workspace boundary properties:**

| Property | Status |
|---|---|
| Two workspaces share no files on disk | ✅ Enforced by path layout. |
| `permissions.toml` is per-workspace; one workspace cannot read another's bindings | ✅ Tested in `liquid-permissions::FilesystemPermissionIndex` integration tests. |
| Tokens are not scoped to a single workspace (per ADR-002) | ⚠️ Intentional. Authorisation per request is enforced by `require_permission!` taking `(WorkspaceId, Action, Resource)` together. A token doesn't open a workspace; the permission index does. |
| Cross-workspace data flow via the SlotBroker | ⏳ Phase 2 — when it lands, it carries the publisher's `WorkspaceId` and a typed scope (CLAUDE.md Absolute Rule 3 governs the design). |

### Threats

| Threat | Mitigation |
|---|---|
| Symlink escape from `<root>/workspaces/<id>/files/` | **Not currently mitigated.** `FilesystemContentStore` opens files by relative path under the workspace root, but does not call `realpath`-style resolution. A malicious operator with filesystem write access could plant a symlink. Roadmap: refuse to follow symlinks on writes / reads (TASK to file before M6.5 lands). |
| TOCTOU between permission check and write | Permission check happens in the same call as the write (`require_permission!` is the *first* line of every bridge fn). The write itself is atomic via tmp-then-rename. The principal's role assignments are only re-read on next call — so a freshly-revoked role can still complete one in-flight write. Documented as acceptable in ADR-002. |
| Path traversal via crafted `StorePath` | Validated on construction; no `..`, no absolute paths, no empty segments. Integration-tested. |
| Workspace id collision | `WorkspaceId` is a `Uuid::new_v4`; collision probability is negligible. |

## 5. FFI permission gate

Absolute Rule 4 (`CLAUDE.md`): every `liquid-sdk-bridge` FFI function
calls `require_permission!(index, principal, action, resource)` *before
any other logic*. The macro expands to:

```rust
if !index.check(principal, action, resource)? {
    return Err(LiquidError::Forbidden);
}
```

This is enforced by review, not by the compiler. The `code-reviewer`
subagent's checklist explicitly lists "permission check is the first
line of every new bridge function".

### Threats

| Threat | Mitigation |
|---|---|
| Forgetting `require_permission!` in a new FFI fn | Review checklist; `code-reviewer` subagent (`.claude/agents/code-reviewer.md`) flags missing gates. Roadmap: a clippy-style lint (TASK to file post-M5). |
| `require_permission!` reading state and acting on stale data | The macro reads `permissions.toml` (or in-memory cache) at call time. If a binding is revoked between two FFI calls, the first call is unaffected, the second is rejected — which is the intended semantics. |
| `LiquidError::Forbidden` leaking which guard tripped | The error variant is `Forbidden` — no detail. Information about *why* a check failed lives only in process logs, never in the FFI return. |

## 6. Agent-CLI misuse

The `liquid` CLI is the agent-facing surface. Agents share the same
permissions model as users, but interact through clap subcommands
instead of a GUI.

| Threat | Mitigation |
|---|---|
| Token leakage via process listing (`ps auxw` shows `--token`) | The `--token` flag is documented as *not recommended* in §12. Tokens are normally read from `LIQUID_TOKEN` env var or `~/.liquid/token` (mode `0600`). |
| Token leakage via shell history | Operators are expected to keep `LIQUID_TOKEN` out of `.bash_history`; the CLI itself does not log tokens. |
| An agent provisioning sub-agents to escalate scope | Only the `WorkspaceOwner` role permits `auth provision-agent`. `AppEditor` and `AppViewer` agents cannot create new agents (M3 permission matrix). |
| An agent reading data outside its assigned workspace/scope | Every read goes through `require_permission!` with `(WorkspaceId, Read, Resource)`. The `Resource` for an `AppViewer` is scoped to a specific app instance (M3 `assign_role` validates scope-required roles carry `Some(Resource)`). |
| An agent issuing a flood of write/undo cycles to thrash the op log | **Not currently rate-limited.** The op_log grows unboundedly. Roadmap: per-principal rate limit + op-log compaction (Phase 3). For Phase 1, the workspace owner is the only operator and is expected to monitor disk usage. |
| An agent uploading a malicious app manifest | Phase-1 stub fails *open* with a warning (per §5.9 exit criteria). Phase 2 makes manifest verification fail closed; the threat model gets a new section then. |
| An agent invoking `liquid auth login` to capture a user password | `auth login` is interactive; agents do not have a TTY. Phase 3 adds an OIDC redirect that explicitly *cannot* be driven by an agent. |

## 7. What this model is silent about (Phase 1 deliberate gaps)

- **At-rest encryption.** No, today. The OS-level filesystem perms are
  the only barrier. If the workspace contains sensitive data, the
  operator is responsible for full-disk encryption.
- **Audit-log integrity.** `op_log.jsonl` is append-only by convention
  but not cryptographically chained. A privileged attacker on the
  workstation can rewrite history without detection. Roadmap: optional
  hash-chain (Phase 3+).
- **Network attacks.** Phase 1 has no network listeners. When Phase 3
  introduces Redis/Redpanda, this document gets a §8.
- **Denial of service.** No rate limits today. Disk-fill is the obvious
  vector; the M6.5 audit-list command is read-only and constant-stream
  so it is itself not a DoS vector against the server (there is no
  server).
- **Supply-chain attacks.** Covered by `deny.toml` and `cargo audit`
  for Rust deps (commit added in the previous PR). App-manifest signing
  is Phase 2.
- **Cross-app data exfiltration.** When the SlotBroker lands (Phase 2),
  this document gets a §9 covering it. For now, Liquid has no apps to
  exfiltrate from.

## 8. Reporting issues

Pre-1.0 there is no SLA. Report via GitHub Security Advisories on the
repository — see [`SECURITY.md`](../../SECURITY.md). The post-1.0
disclosure policy is tracked in
[`IMPLEMENTATION_PLAN.md`](../../IMPLEMENTATION_PLAN.md) §17.3.
