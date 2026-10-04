//! Machine-readable failures.
//!
//! Retryability and the HTTP status are documented per variant; a transport
//! maps them, this crate does not.

use std::time::Duration;

use crate::id::SnapshotId;
use crate::sandbox::SandboxState;

/// A service failure. A command's non-zero exit is not one of these.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// No such sandbox, execution or operation. HTTP 404, not retryable.
    #[error("{resource} not found")]
    NotFound { resource: String },

    /// Malformed request. HTTP 400, not retryable.
    #[error("invalid {field}: {message}")]
    InvalidArgument { field: String, message: String },

    /// A path resolved outside the workspace. HTTP 400, not retryable.
    #[error("path {path} escapes the workspace")]
    PathEscape { path: String },

    /// The runtime cannot provide this capability. HTTP 412, not retryable.
    #[error("unsupported capability: {name}")]
    UnsupportedCapability { name: String },

    /// A snapshot does not match the runtime or configuration. HTTP 412, not
    /// retryable.
    #[error("snapshot {snapshot:?} is incompatible: {reason}")]
    SnapshotIncompatible {
        snapshot: SnapshotId,
        reason: String,
    },

    /// An idempotency key was reused with a different payload. HTTP 409, not
    /// retryable.
    #[error("idempotency key {key} was reused with a different payload")]
    IdempotencyConflict { key: String },

    /// The lifecycle state forbids this request. HTTP 409, retry after reading
    /// the sandbox's status.
    #[error("{requested} is not allowed while the sandbox is {state:?}")]
    TransitionConflict {
        state: SandboxState,
        requested: String,
    },

    /// A drain did not complete before its deadline. HTTP 409, retryable.
    #[error("busy: {reason}")]
    Busy { reason: String },

    /// A request body exceeded its size cap. HTTP 413, not retryable.
    #[error("{size} bytes exceeds the {max}-byte limit")]
    FileTooLarge { size: u64, max: u64 },

    /// The request backlog is full. HTTP 429, retryable; honor `retry_after`.
    #[error("capacity exhausted; retry after {retry_after:?}")]
    CapacityExhausted { retry_after: Duration },

    /// The runtime or engine is not usable now. HTTP 503, retryable.
    #[error("runtime unavailable: {reason}")]
    RuntimeUnavailable { reason: String },

    /// The event cursor predates the retained window. HTTP 410, not retryable.
    #[error("cursor expired; the earliest retained sequence is {earliest}")]
    CursorExpired { earliest: u64 },

    /// A bug or an unhandled state. HTTP 500.
    #[error("internal error: {message}")]
    Internal { message: String },
}
