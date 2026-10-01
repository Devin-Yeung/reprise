//! Commands and their observable output.

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

use crate::error::Error;
use crate::id::{ExecutionId, IdempotencyKey, SandboxId};

/// The state of one command execution.
///
/// `Exited` is the normal terminal state and carries an exit code that may be
/// non-zero. `Failed` means the service could not run or could not observe the
/// command, and pairs with [`Execution::outcome_unknown`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExecutionState {
    /// Recorded, not yet running. The sandbox may still be activating, or the
    /// execution may be waiting for admission.
    Accepted,
    /// Started inside the sandbox.
    Running,
    /// The process exited. `exit_code` is set and may be non-zero.
    Exited,
    /// Cancellation ended the process, or a forced suspend did.
    Canceled,
    /// The command's own deadline ended it after TERM and KILL.
    TimedOut,
    /// The service could not run, or could not observe, the command.
    Failed,
}

/// Which stream a chunk of output came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Channel {
    Stdout,
    Stderr,
}

/// One command execution as stored and observed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Execution {
    pub id: ExecutionId,
    pub sandbox: SandboxId,
    pub state: ExecutionState,
    /// Set in [`ExecutionState::Exited`]. A non-zero value is a result, not an
    /// [`Error`].
    pub exit_code: Option<i32>,
    /// True once output hit its retention cap and older events were dropped.
    pub output_truncated: bool,
    /// True when the service cannot determine whether the command ran. Such an
    /// execution is never retried automatically.
    pub outcome_unknown: bool,
    pub error: Option<Error>,
    pub created_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub finished_at: Option<SystemTime>,
}

/// Parameters for running a command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Execute {
    /// A typed argument vector. There is no implicit shell; a caller that wants
    /// shell semantics sends `["sh", "-lc", script]`.
    pub argv: Vec<String>,
    /// Workspace-relative working directory.
    pub cwd: Option<String>,
    /// Explicit environment overrides, layered over the template defaults.
    pub env: BTreeMap<String, String>,
    /// Optional, bounded standard input. Interactive terminals are out of
    /// scope.
    pub stdin: Option<Vec<u8>>,
    /// Ends the command with [`ExecutionState::TimedOut`] after TERM and KILL.
    pub timeout: Option<Duration>,
    pub idempotency_key: Option<IdempotencyKey>,
}

/// One item in an execution's output log.
///
/// `sequence` is per-execution, strictly increasing and gap-free. It is the
/// cursor a caller resumes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionEvent {
    Output {
        sequence: u64,
        channel: Channel,
        chunk: Vec<u8>,
    },
    /// A state change. The terminal status is also readable from the
    /// [`Execution`] itself.
    Status {
        sequence: u64,
        state: ExecutionState,
        exit_code: Option<i32>,
    },
    /// Output retention dropped older events; reading before this point is no
    /// longer possible.
    Truncated { sequence: u64 },
}

/// A resumable page of [`ExecutionEvent`]s.
///
/// Poll with [`EventPage::next`] as the next `after` cursor. Read the
/// [`Execution`] to learn whether it has reached a terminal state; an empty
/// page does not mean the execution has finished.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventPage {
    pub events: Vec<ExecutionEvent>,
    /// The cursor to pass as `after` on the next call.
    pub next: u64,
}
