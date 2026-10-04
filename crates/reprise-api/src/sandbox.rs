//! Sandboxes: stable identity, lifecycle state and static configuration.

use std::time::{Duration, SystemTime};

use crate::error::Error;
use crate::id::{ExecutionId, IdempotencyKey, OperationId, SandboxId, SnapshotId, TemplateId};

/// The lifecycle state of a sandbox.
///
/// `Suspended` and `Running` are the resting states. `Resuming` and
/// `Suspending` are committed transitions in progress. `Failed` means the
/// physical state could not be confirmed; [`Sandbox::latest_snapshot`] keeps
/// the last known good snapshot, and the sandbox is never resumed into an
/// unknown second instance. `Deleting` is teardown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SandboxState {
    /// No active instance. A resume activates from a committed snapshot, or
    /// cold-boots if none exists. The state of a freshly created sandbox.
    Suspended,
    /// Activation committed and in progress.
    Resuming,
    /// An active instance holds the sandbox and its workspace.
    Running,
    /// Checkpoint-and-release in progress.
    Suspending,
    /// The physical state could not be confirmed.
    Failed,
    /// Teardown in progress.
    Deleting,
}

/// Requested resource limits for a sandbox.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Limits {
    /// CPU in millicores, when requested.
    pub cpu_millicores: Option<u32>,
    /// Memory in bytes, when requested.
    pub memory_bytes: Option<u64>,
}

/// A sandbox as stored and observed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sandbox {
    pub id: SandboxId,
    pub template: TemplateId,
    pub state: SandboxState,
    /// Incremented on every successful activation. Lets a caller tell that the
    /// physical instance changed across a suspend/resume.
    pub generation: u64,
    pub limits: Limits,
    /// Auto-suspend a sandbox with no running work after this interval, when
    /// set. A policy hint, not a guarantee.
    pub idle_timeout: Option<Duration>,
    /// The most recent committed snapshot, if any.
    pub latest_snapshot: Option<SnapshotId>,
    pub created_at: SystemTime,
}

/// A [`Sandbox`] plus the live facts a caller needs to reason about it now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxInfo {
    pub sandbox: Sandbox,
    /// Executions that have been accepted and have not reached a terminal
    /// state.
    pub active_executions: Vec<ExecutionId>,
    /// The transition currently in progress, if any.
    pub current_operation: Option<OperationId>,
    pub last_error: Option<Error>,
}

/// Parameters for creating a sandbox.
///
/// Registration only: no runtime starts, no image is pulled and no snapshot is
/// taken. The template's image is resolved lazily on first activation.
///
/// Construct with `CreateSandbox::builder()`. The template is required. Limits
/// default to unset CPU and memory; the idle timeout and idempotency key
/// default to `None`.
///
/// ```compile_fail
/// use reprise_api::CreateSandbox;
///
/// // A template must be supplied before building.
/// let _request = CreateSandbox::builder().build();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct CreateSandbox {
    pub template: TemplateId,
    /// Requested resource limits. Defaults to unset CPU and memory.
    #[builder(default)]
    pub limits: Limits,
    /// Auto-suspend a sandbox with no running work after this interval, when
    /// set. A policy hint, not a guarantee. Defaults to `None`.
    #[builder(default, setter(strip_option, into))]
    pub idle_timeout: Option<Duration>,
    /// Optional deduplication key. Defaults to `None`.
    #[builder(default, setter(strip_option, into))]
    pub idempotency_key: Option<IdempotencyKey>,
}
