use bytes::Bytes;
use liquid_core::{CommitId, OperationId, PrincipalId, StorePath};
use serde::{Deserialize, Serialize};

/// One entry in a workspace's append-only operation log.
///
/// Mirrors the shape of a Jujutsu `op log` entry. `OperationKind` carries
/// enough information to invert the operation later via
/// [`crate::ContentStore::undo`] without consulting the underlying store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: OperationId,
    pub commit: CommitId,
    pub timestamp_unix_millis: u64,
    pub author: PrincipalId,
    pub message: String,
    pub kind: OperationKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum OperationKind {
    /// First write of `path`. Inversion: remove `path`.
    Create { path: StorePath, content: Bytes },

    /// Overwrite of an existing `path`. Inversion: restore `prev`.
    Update {
        path: StorePath,
        prev: Bytes,
        content: Bytes,
    },

    /// Deletion of `path`. Inversion: restore `prev`.
    Delete { path: StorePath, prev: Bytes },

    /// Synthetic record produced by [`crate::ContentStore::undo`].
    /// Carries the id of the operation that was inverted; not itself
    /// reversible.
    Undo { target: OperationId },
}
