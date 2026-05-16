//! Thin FFI surface bridging Dart (Flutter) and the Rust core.
//!
//! Per `IMPLEMENTATION_PLAN.md §5.5` and ADR-004, this crate is the
//! single permission-enforcement point: every public function on
//! [`BridgeServices`] validates the caller's token (collapsing
//! failures to `LiquidError::Forbidden` per §4.5) and runs
//! `require_permission!` *before* any other logic. No business logic
//! lives here — it delegates to `liquid-vcs`, `liquid-auth`,
//! `liquid-permissions`, and the local [`WorkspaceRegistry`].
//!
//! Phase-1 ships the Rust side only. The Dart side
//! (`flutter_rust_bridge` codegen + `app/lib/bridge/*` + the
//! `flutter test` integration suite the §5.5 success criterion
//! describes) lands once M6 scaffolds `app/` and `sdk/liquid_sdk/`
//! — see `docs/manual-validation-m4-m5.md` §M5 for the PR-review
//! checklist.

pub mod api;
pub mod registry;
pub mod services;
pub mod types;

pub use registry::{
    FilesystemWorkspaceRegistry, InMemoryWorkspaceRegistry, WorkspaceRecord, WorkspaceRegistry,
};
pub use services::BridgeServices;
pub use types::{PageSnapshot, WorkspaceSummary};
