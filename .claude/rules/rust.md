---
paths:
  - "core/**/*.rs"
  - "core/**/Cargo.toml"
  - "core/Cargo.lock"
  - "rustfmt.toml"
---

# Rust Rules

- Prefer small, type-driven changes.
- Run `cargo fmt --manifest-path core/Cargo.toml --all` (or `just fmt-rust`).
- Run focused `cargo test -p <crate> --manifest-path core/Cargo.toml <test_name>` where possible.
- Use `cargo check --manifest-path core/Cargo.toml` to catch type errors early.
- Use `cargo clippy --manifest-path core/Cargo.toml --workspace --all-targets --locked -- -D warnings` for non-trivial changes (matches CI).
- Toolchain is pinned in `core/rust-toolchain.toml` (currently `1.94.1`). CI pins to the same version. Do not bump one without the other.
- Preserve public API compatibility of `liquid-core`, `liquid-vcs`, `liquid-auth`,
  `liquid-permissions`, `liquid-cache`, `liquid-bindings`, `liquid-sdk-bridge`,
  and `liquid-cli` unless explicitly asked.
- Avoid unnecessary cloning, allocation, panics, and broad lifetime rewrites.
- Handle errors explicitly via `Result` and preserve existing error-handling
  conventions (`thiserror`).
- **No `unsafe` code.** The workspace forbids `unsafe_code` (`core/Cargo.toml`
  workspace lints). Do not introduce `unsafe` blocks or features that require it.
- **No `unwrap()` / `expect()` outside `#[cfg(test)]`.** This is a project
  Absolute Rule (`CLAUDE.md`).
- For async code, preserve runtime (`tokio`) and cancellation conventions.
- For serialization, preserve existing `serde` schema compatibility.
- Per `CLAUDE.md` Absolute Rule 4, every `liquid-sdk-bridge` FFI function must
  call `require_permission!` before any other logic.
- Per `CLAUDE.md` Absolute Rule 5, every storage call takes a `WorkspaceId`.
