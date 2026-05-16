//! Content-addressable read cache.
//!
//! Implements [`ReadCache`] (defined by `IMPLEMENTATION_PLAN.md` §4.3,
//! milestone M4 in §5.4) and ships one Phase-1 backend,
//! [`InProcessCache`], a `Arc<DashMap<ContentHash, Bytes>>` with no
//! expiry (§9).
//!
//! Phase 3 adds `RedisCache` behind a `distributed-cache` feature flag
//! per §9. The trait is intentionally minimal so the swap is
//! transparent to callers (`liquid-vcs` wraps the trait, not a concrete
//! type).

pub mod in_process;
pub mod read_cache;

pub use in_process::InProcessCache;
pub use read_cache::ReadCache;
