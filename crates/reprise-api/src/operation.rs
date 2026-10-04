//! Lifecycle transitions and their progress.
//!
//! An operation is accepted once and then runs to completion. It is not
//! canceled when the caller disconnects, and it is not canceled by
//! [`Destroy::force`] or [`Suspend::force`], which cancel *executions* instead.

use std::time::SystemTime;

use crate::error::Error;
use crate::id::{IdempotencyKey, OperationId, SandboxId, SnapshotId};

/// Which transition an operation performs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OperationKind {
    Suspend,
    Resume,
    Destroy,
}

/// Progress of a lifecycle operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationState {
    /// Accepted, not yet started.
    Pending,
    /// In progress.
    Running,
    /// Completed. The result variant matches the operation's
    /// [`OperationKind`].
    Succeeded(OperationResult),
    /// Did not complete.
    Failed(Error),
}

/// The outcome of a successful operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationResult {
    /// The sandbox is suspended and the committed snapshot is named.
    Suspend { snapshot: SnapshotId },
    /// The sandbox is running; `generation` is the new instance's, and
    /// `snapshot` is what it was restored from, if any.
    Resume {
        snapshot: Option<SnapshotId>,
        generation: u64,
    },
    /// The sandbox and its snapshots are gone.
    Destroy,
}

/// One lifecycle transition as stored and observed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation {
    pub id: OperationId,
    pub kind: OperationKind,
    pub sandbox: SandboxId,
    pub state: OperationState,
    /// A human-readable label for the current step, for progress reporting.
    pub step: Option<String>,
    pub created_at: SystemTime,
    pub finished_at: Option<SystemTime>,
}

/// Request the full checkpoint-and-release transition.
///
/// On success the sandbox is [`Suspended`](crate::SandboxState::Suspended), the
/// runtime is freed, and the result carries the committed [`SnapshotId`]. The
/// new snapshot is written as a candidate, verified and committed atomically;
/// the previous committed snapshot is untouched until then. If the commit fails
/// the sandbox is [`Failed`](crate::SandboxState::Failed) and the previous
/// snapshot is retained.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Suspend {
    /// Cancel in-flight executions instead of waiting for them to drain.
    pub force: bool,
    pub idempotency_key: Option<IdempotencyKey>,
}

/// Request activation.
///
/// Activation uses the committed snapshot when one exists and cold-boots from
/// the template otherwise; the caller does not choose. On an already
/// `Resuming` sandbox, the in-flight operation is joined.
///
/// Construct with `Resume::builder()`. The idempotency key defaults to `None`.
///
/// ```
/// use reprise_api::Resume;
///
/// let request = Resume::builder().build();
///
/// assert!(request.idempotency_key.is_none());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct Resume {
    /// Optional deduplication key. Defaults to `None`.
    #[builder(default, setter(strip_option, into))]
    pub idempotency_key: Option<IdempotencyKey>,
}

/// Request teardown.
///
/// On success the sandbox is terminal, its runtime is gone and its snapshots
/// are garbage collected. Reachable from any state.
///
/// Construct with `Destroy::builder()`. `force` defaults to `false`, so
/// in-flight executions drain. The idempotency key defaults to `None`.
///
/// ```
/// use reprise_api::Destroy;
///
/// let request = Destroy::builder().force(true).build();
///
/// assert!(request.force);
/// assert!(request.idempotency_key.is_none());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct Destroy {
    /// Cancel in-flight executions instead of waiting for them to drain.
    /// Defaults to `false`.
    #[builder(default)]
    pub force: bool,
    /// Optional deduplication key. Defaults to `None`.
    #[builder(default, setter(strip_option, into))]
    pub idempotency_key: Option<IdempotencyKey>,
}
