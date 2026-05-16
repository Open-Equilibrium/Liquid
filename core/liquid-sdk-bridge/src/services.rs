//! The composition root for the Rust-side bridge.
//!
//! [`BridgeServices`] bundles every Phase-1 backend (`ContentStore`,
//! `PermissionIndex`, `IdentityProvider`, `WorkspaceRegistry`) so the
//! 5 §5.5 FFI entry points have a single `&self` to read from.
//!
//! Generic over the four trait shapes so test code can wire in
//! `InMemory*` variants while a production deployment wires in
//! `FilesystemContentStore` + `FilesystemPermissionIndex` +
//! `LocalIdentityProvider` + (future) `FilesystemWorkspaceRegistry`.

use std::sync::Arc;

use liquid_auth::IdentityProvider;
use liquid_permissions::PermissionIndex;
use liquid_vcs::ContentStore;

use crate::registry::WorkspaceRegistry;

/// Composition root. Construct one per process and share via `Arc`.
///
/// All four fields are `pub` because tests + production set-up code
/// build the bundle directly; there is no constructor convention to
/// enforce. Phase 3 may add a builder once the dependency set grows.
pub struct BridgeServices<S, P, I, R>
where
    S: ContentStore,
    P: PermissionIndex,
    I: IdentityProvider,
    R: WorkspaceRegistry,
{
    pub store: Arc<S>,
    pub permissions: Arc<P>,
    pub identity: Arc<I>,
    pub registry: Arc<R>,
}
