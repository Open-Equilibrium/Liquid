//! Versioned content store for Liquid.
//!
//! Defines the [`ContentStore`] trait (originally specified in
//! `IMPLEMENTATION_PLAN.md` §4.1) and ships two implementations:
//!
//! - [`InMemoryContentStore`] — test/dev backend, no persistence.
//! - [`FilesystemContentStore`] — durable on-disk backend used in Phase 1
//!   (see `docs/adr/001-jujutsu-pinning.md`).
//!
//! The `jj-lib`-backed `JujutsuContentStore` is deferred to TASK-004 per
//! ADR-001. Application code only ever sees the trait, so the swap is
//! transparent.

pub mod content_store;
pub mod filesystem;
pub mod in_memory;
pub mod operation;

pub use content_store::ContentStore;
pub use filesystem::FilesystemContentStore;
pub use in_memory::InMemoryContentStore;
pub use operation::{Operation, OperationKind};
