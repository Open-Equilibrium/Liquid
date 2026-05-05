//! Identity and session management.
//!
//! Phase 1 ships the [`LocalIdentityProvider`] — a TOML-backed user/agent
//! store with Argon2id-hashed passwords and HMAC-SHA256 session tokens
//! (`IMPLEMENTATION_PLAN.md` §5.3, §9). Phase 3 will add OIDC; callers
//! depend on the [`IdentityProvider`] trait so the swap is transparent.

pub mod local;
pub mod provider;
mod storage;
mod token;

pub use local::LocalIdentityProvider;
pub use provider::IdentityProvider;
