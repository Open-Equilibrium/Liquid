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

/// One entry in the operation log.
///
/// **Phase-1 storage caveat.** `Update` and `Delete` variants carry
/// the pre-image bytes inline so `undo` can reverse them without a
/// second store lookup. With the JSONL on-disk encoding (one record
/// per line in `op_log.jsonl`), each `Update` of an N-byte file
/// appends roughly `2 × N` bytes of `prev` + `content` serialized as
/// JSON number arrays — ~3-4× the raw size. A workspace with many
/// rewrites of large pages grows the log without bound and
/// `FilesystemContentStore::undo` re-reads the whole file on every
/// call. Content-addressed pre-image storage + a periodic log
/// compaction land with the M2 Jujutsu backend (TASK-004,
/// `IMPLEMENTATION_PLAN.md §5.2` final sub-task), which is the
/// designed-in fix for this growth pattern; the in-memory and
/// JSONL backends are explicitly the development / single-user
/// variants per ADR-001.
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
