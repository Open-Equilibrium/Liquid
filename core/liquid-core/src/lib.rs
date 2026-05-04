//! Liquid core primitives.
//!
//! Shared identifiers, error types, and value types used by every other crate
//! in the workspace. No I/O, no async, no business logic. See
//! `IMPLEMENTATION_PLAN.md` §9 (`liquid-core` reference).

pub mod content_hash;
pub mod error;
pub mod ids;
pub mod permission;
pub mod slot;
pub mod store_path;
pub mod tenant;

pub use content_hash::ContentHash;
pub use error::{LiquidError, Result};
pub use ids::{
    AppInstanceId, CommitId, ComponentId, OperationId, PageId, PrincipalId, RoleId, WorkspaceId,
};
pub use permission::{Action, Resource};
pub use slot::{SlotName, SlotValue};
pub use store_path::StorePath;
pub use tenant::TenantConfig;
