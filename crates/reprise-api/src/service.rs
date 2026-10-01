//! The single public seam.
//!
//! Everything above a [`SandboxService`] — a CLI, an MCP adapter, an HTTP
//! server — crosses this trait and reimplements no lifecycle logic.

use crate::capabilities::Capabilities;
use crate::error::Error;
use crate::execution::{EventPage, Execute, Execution};
use crate::files::{FileContent, FileVersion, ReadFile, WriteFile};
use crate::id::{ExecutionId, OperationId, SandboxId};
use crate::operation::{Destroy, Operation, Resume, Suspend};
use crate::sandbox::{CreateSandbox, Sandbox, SandboxInfo};

/// The only interface callers use.
///
/// # Contracts shared by every method
///
/// - **Stable identity.** A [`SandboxId`] outlives every physical instance
///   behind it; [`Sandbox::generation`] changes when that instance changes.
/// - **Wake on work.** Callers never resume before using a sandbox. Any command
///   or file operation on a `Suspended` sandbox activates it through the same
///   resume path, and concurrent activations of one sandbox collapse into one.
/// - **Accept, then admit.** A request is accepted before capacity or restore
///   completes. Accepted work is not canceled because the caller stopped
///   waiting.
/// - **A running execution holds a pin.** Suspend drains by default; `force`
///   cancels executions instead.
/// - **Idempotency.** A mutating request may carry an
///   [`IdempotencyKey`](crate::IdempotencyKey); the same key with the same
///   payload returns the original resource, and the same key with a different
///   payload is [`Error::IdempotencyConflict`].
#[allow(async_fn_in_trait)]
pub trait SandboxService: Send + Sync {
    /// Register a sandbox and return it in
    /// [`Suspended`](crate::SandboxState::Suspended). No runtime starts.
    async fn create(&self, request: CreateSandbox) -> Result<Sandbox, Error>;

    /// Return the sandbox and its live facts. This is the call a client polls
    /// when it is not observing an operation or an execution.
    async fn inspect(&self, sandbox: &SandboxId) -> Result<SandboxInfo, Error>;

    /// Accept a command and return it in
    /// [`Accepted`](crate::ExecutionState::Accepted). It runs only once the
    /// sandbox is running and the execution is admitted; both are observable
    /// through [`get_execution`](Self::get_execution) and
    /// [`execution_events`](Self::execution_events). A request that cannot be
    /// queued is refused with [`Error::CapacityExhausted`].
    async fn execute(&self, sandbox: &SandboxId, request: Execute) -> Result<Execution, Error>;

    /// Request cancellation of an execution: TERM, then KILL after a grace
    /// period. Idempotent; canceling a terminal execution returns its existing
    /// terminal state.
    async fn cancel(&self, execution: &ExecutionId) -> Result<Execution, Error>;

    /// Read a workspace file, activating the sandbox if needed.
    async fn read_file(&self, sandbox: &SandboxId, request: ReadFile)
    -> Result<FileContent, Error>;

    /// Write a workspace file atomically, activating the sandbox if needed.
    async fn write_file(
        &self,
        sandbox: &SandboxId,
        request: WriteFile,
    ) -> Result<FileVersion, Error>;

    /// Request checkpoint-and-release. Drains by default; fails
    /// [`Error::Busy`] and returns the sandbox to `Running` if drain cannot
    /// complete.
    async fn suspend(&self, sandbox: &SandboxId, request: Suspend) -> Result<Operation, Error>;

    /// Request activation from the committed snapshot, or a cold boot when
    /// none exists. Joins an in-flight resume.
    async fn resume(&self, sandbox: &SandboxId, request: Resume) -> Result<Operation, Error>;

    /// Request teardown, which also garbage-collects snapshots.
    async fn destroy(&self, sandbox: &SandboxId, request: Destroy) -> Result<Operation, Error>;

    /// Read a lifecycle operation as of now.
    async fn get_operation(&self, operation: &OperationId) -> Result<Operation, Error>;

    /// Read an execution as of now.
    async fn get_execution(&self, execution: &ExecutionId) -> Result<Execution, Error>;

    /// Read events after `after`, up to `limit`. Poll with
    /// [`EventPage::next`]; a cursor older than the retained window is
    /// [`Error::CursorExpired`].
    async fn execution_events(
        &self,
        execution: &ExecutionId,
        after: u64,
        limit: usize,
    ) -> Result<EventPage, Error>;

    /// Report what the configured runtime has proven it can do.
    async fn capabilities(&self) -> Capabilities;
}
