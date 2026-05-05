//! RBAC model and materialised permission index.
//!
//! Implements the [`PermissionIndex`] trait (specified in
//! `IMPLEMENTATION_PLAN.md` §4.2) plus an in-memory backend
//! ([`InMemoryPermissionIndex`]) that ships with M3 (§5.3). The role →
//! permission matrix is hard-coded in [`BuiltInRole::permits`]; per §9 it
//! becomes runtime-configurable in Phase 3.
//!
//! The [`require_permission!`] macro is the canonical permission gate for
//! every `liquid-sdk-bridge` and CLI entrypoint (CLAUDE.md rule 4).

pub mod index;
pub mod macros;
pub mod role;

pub use index::{InMemoryPermissionIndex, PermissionIndex};
pub use role::BuiltInRole;

#[doc(hidden)]
pub mod __macro_support {
    pub use liquid_core::{LiquidError, Resource};
}
