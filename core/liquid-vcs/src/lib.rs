//! Versioned content store for Liquid.
//!
//! Defines the [`ContentStore`] trait (originally specified in
//! `IMPLEMENTATION_PLAN.md` §4.1) and the in-memory implementation used by
//! tests and Phase 1 dev mode. The Jujutsu-backed implementation
//! ([`docs/adr/001-jujutsu-pinning.md`]) lands in TASK-003.

pub mod content_store;
pub mod in_memory;
pub mod operation;

pub use content_store::ContentStore;
pub use in_memory::InMemoryContentStore;
pub use operation::{Operation, OperationKind};
