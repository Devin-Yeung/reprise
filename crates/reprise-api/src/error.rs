//! Machine-readable failures.
//!
//! Retryability and the HTTP status are documented per variant; a transport
//! maps them, this crate does not.

use std::time::Duration;

use crate::id::SnapshotId;
use crate::sandbox::SandboxState;

/// A service failure. A command's non-zero exit is not one of these.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// No such sandbox, execution or operation. HTTP 404, not retryable.
    NotFound { resource: String },

    /// Malformed request. HTTP 400, not retryable.
    InvalidArgument { field: String, message: String },

    /// A path resolved outside the workspace. HTTP 400, not retryable.
    PathEscape { path: String },

    /// The runtime cannot provide this capability. HTTP 412, not retryable.
    UnsupportedCapability { name: String },

    /// A snapshot does not match the runtime or configuration. HTTP 412, not
    /// retryable.
    SnapshotIncompatible {
        snapshot: SnapshotId,
        reason: String,
    },

    /// An idempotency key was reused with a different payload. HTTP 409, not
    /// retryable.
    IdempotencyConflict { key: String },

    /// The lifecycle state forbids this request. HTTP 409, retry after reading
    /// the sandbox's status.
    TransitionConflict {
        state: SandboxState,
        requested: String,
    },

    /// A drain did not complete before its deadline. HTTP 409, retryable.
    Busy { reason: String },

    /// A request body exceeded its size cap. HTTP 413, not retryable.
    FileTooLarge { size: u64, max: u64 },

    /// The request backlog is full. HTTP 429, retryable; honor `retry_after`.
    CapacityExhausted { retry_after: Duration },

    /// The runtime or engine is not usable now. HTTP 503, retryable.
    RuntimeUnavailable { reason: String },

    /// The event cursor predates the retained window. HTTP 410, not retryable.
    CursorExpired { earliest: u64 },

    /// A bug or an unhandled state. HTTP 500.
    Internal { message: String },
}
