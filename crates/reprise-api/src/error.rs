//! Machine-readable failures.
//!
//! Retryability and the HTTP status are documented per variant; a transport
//! maps them, this crate does not.

use std::time::Duration;

use crate::capabilities::Capability;
use crate::id::{ExecutionId, IdempotencyKey, OperationId, SandboxId, SnapshotId};
use crate::operation::OperationKind;
use crate::sandbox::SandboxState;

/// A service failure. A command's non-zero exit is not one of these.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// No such sandbox, execution or operation. HTTP 404, not retryable.
    #[error("{resource} not found")]
    NotFound { resource: Resource },

    /// Malformed request. HTTP 400, not retryable.
    #[error("invalid {field}: {message}")]
    InvalidArgument { field: String, message: String },

    /// A path resolved outside the workspace. HTTP 400, not retryable.
    #[error("path {path} escapes the workspace")]
    PathEscape { path: String },

    /// The runtime cannot provide this capability. HTTP 412, not retryable.
    #[error("unsupported capability: {capability:?}")]
    UnsupportedCapability { capability: Capability },

    /// A snapshot does not match the runtime or configuration. HTTP 412, not
    /// retryable.
    #[error("snapshot {snapshot} is incompatible: {reason}")]
    SnapshotIncompatible {
        snapshot: SnapshotId,
        reason: String,
    },

    /// An idempotency key was reused with a different payload. HTTP 409, not
    /// retryable.
    #[error("idempotency key {key} was reused with a different payload")]
    IdempotencyConflict { key: IdempotencyKey },

    /// The lifecycle state forbids this request. HTTP 409, retry after reading
    /// the sandbox's status.
    #[error("{requested:?} is not allowed while the sandbox is {state:?}")]
    TransitionConflict {
        state: SandboxState,
        requested: RequestKind,
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

/// The resource an [`Error::NotFound`] refers to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resource {
    Sandbox(SandboxId),
    Execution(ExecutionId),
    Operation(OperationId),
}

impl std::fmt::Display for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sandbox(id) => write!(f, "sandbox {id}"),
            Self::Execution(id) => write!(f, "execution {id}"),
            Self::Operation(id) => write!(f, "operation {id}"),
        }
    }
}

/// A sandbox request that the lifecycle state can forbid, as reported by
/// [`Error::TransitionConflict`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RequestKind {
    Execute,
    ReadFile,
    WriteFile,
    Suspend,
    Resume,
    Destroy,
}

impl From<OperationKind> for RequestKind {
    fn from(kind: OperationKind) -> Self {
        match kind {
            OperationKind::Suspend => Self::Suspend,
            OperationKind::Resume => Self::Resume,
            OperationKind::Destroy => Self::Destroy,
        }
    }
}
