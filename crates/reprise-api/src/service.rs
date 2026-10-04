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
/// Every returned future is `Send`, so calls can be spawned onto a
/// multi-threaded runtime. Implementations may write `async fn`.
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
pub trait SandboxService: Send + Sync {
    /// Register a sandbox and return it in
    /// [`Suspended`](crate::SandboxState::Suspended). No runtime starts.
    fn create(&self, request: CreateSandbox)
    -> impl Future<Output = Result<Sandbox, Error>> + Send;

    /// Return the sandbox and its live facts. This is the call a client polls
    /// when it is not observing an operation or an execution.
    fn inspect(
        &self,
        sandbox: &SandboxId,
    ) -> impl Future<Output = Result<SandboxInfo, Error>> + Send;

    /// Accept a command and return it in
    /// [`Accepted`](crate::ExecutionState::Accepted). It runs only once the
    /// sandbox is running and the execution is admitted; both are observable
    /// through [`get_execution`](Self::get_execution) and
    /// [`execution_events`](Self::execution_events). A request that cannot be
    /// queued is refused with [`Error::CapacityExhausted`].
    fn execute(
        &self,
        sandbox: &SandboxId,
        request: Execute,
    ) -> impl Future<Output = Result<Execution, Error>> + Send;

    /// Request cancellation of an execution: TERM, then KILL after a grace
    /// period. Idempotent; canceling a terminal execution returns its existing
    /// terminal state.
    fn cancel(
        &self,
        execution: &ExecutionId,
    ) -> impl Future<Output = Result<Execution, Error>> + Send;

    /// Read a workspace file, activating the sandbox if needed.
    fn read_file(
        &self,
        sandbox: &SandboxId,
        request: ReadFile,
    ) -> impl Future<Output = Result<FileContent, Error>> + Send;

    /// Write a workspace file atomically, activating the sandbox if needed.
    fn write_file(
        &self,
        sandbox: &SandboxId,
        request: WriteFile,
    ) -> impl Future<Output = Result<FileVersion, Error>> + Send;

    /// Request checkpoint-and-release. Drains by default; fails
    /// [`Error::Busy`] and returns the sandbox to `Running` if drain cannot
    /// complete.
    fn suspend(
        &self,
        sandbox: &SandboxId,
        request: Suspend,
    ) -> impl Future<Output = Result<Operation, Error>> + Send;

    /// Request activation from the committed snapshot, or a cold boot when
    /// none exists. Joins an in-flight resume.
    fn resume(
        &self,
        sandbox: &SandboxId,
        request: Resume,
    ) -> impl Future<Output = Result<Operation, Error>> + Send;

    /// Request teardown, which also garbage-collects snapshots.
    fn destroy(
        &self,
        sandbox: &SandboxId,
        request: Destroy,
    ) -> impl Future<Output = Result<Operation, Error>> + Send;

    /// Read a lifecycle operation as of now.
    fn get_operation(
        &self,
        operation: &OperationId,
    ) -> impl Future<Output = Result<Operation, Error>> + Send;

    /// Read an execution as of now.
    fn get_execution(
        &self,
        execution: &ExecutionId,
    ) -> impl Future<Output = Result<Execution, Error>> + Send;

    /// Read events after `after`, up to `limit`. Poll with
    /// [`EventPage::next`]; a cursor older than the retained window is
    /// [`Error::CursorExpired`].
    fn execution_events(
        &self,
        execution: &ExecutionId,
        after: u64,
        limit: usize,
    ) -> impl Future<Output = Result<EventPage, Error>> + Send;

    /// Report what the configured runtime has proven it can do.
    fn capabilities(&self) -> impl Future<Output = Capabilities> + Send;
}
