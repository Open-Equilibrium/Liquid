//! Wire types exposed across the FFI boundary.
//!
//! Both types are `serde`-friendly so `flutter_rust_bridge` codegen can
//! emit matching Dart classes. They live in the bridge crate, not in
//! `liquid-core`, because they exist purely to satisfy the bridge
//! contract (`IMPLEMENTATION_PLAN.md` §5.5) — `liquid-core` types are
//! lower-level primitives.

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use liquid_core::{ContentHash, PageId, PrincipalId, WorkspaceId};

/// A workspace as the bridge exposes it to UI / agent callers.
///
/// Returned by [`crate::BridgeServices::list_workspaces`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSummary {
    pub id: WorkspaceId,
    pub name: String,
    pub created_by: PrincipalId,
    pub created_unix: u64,
}

/// The bytes + identity of one page at a specific point in time.
///
/// Round-tripped by [`crate::BridgeServices::write_page`] /
/// [`crate::BridgeServices::load_page`]. `content_hash` is computed
/// from `bytes` on construction so a caller cannot supply an
/// inconsistent pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageSnapshot {
    pub page_id: PageId,
    pub bytes: Bytes,
    pub content_hash: ContentHash,
}

impl PageSnapshot {
    /// Construct a snapshot from `page_id` + `bytes`, deriving
    /// `content_hash` so the pair cannot be inconsistent. The Dart side
    /// will receive the same shape from the codegen-emitted constructor.
    #[must_use]
    pub fn new(page_id: PageId, bytes: Bytes) -> Self {
        let content_hash = ContentHash::of_bytes(&bytes);
        Self {
            page_id,
            bytes,
            content_hash,
        }
    }
}
