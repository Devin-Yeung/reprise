//! Commands and their observable output.

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

use crate::error::Error;
use crate::id::{ExecutionId, IdempotencyKey, SandboxId};

/// The state of one command execution.
///
/// `Exited` is the normal terminal state. `Failed` means the service could not
/// run or could not observe the command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionState {
    /// Recorded, not yet running. The sandbox may still be activating, or the
    /// execution may be waiting for admission.
    Accepted,
    /// Started inside the sandbox.
    Running,
    /// The process exited. A non-zero `code` is a result, not an [`Error`].
    Exited { code: i32 },
    /// Cancellation ended the process, or a forced suspend did.
    Canceled,
    /// The command's own deadline ended it after TERM and KILL.
    TimedOut,
    /// The service could not run, or could not observe, the command.
    Failed {
        error: Error,
        /// The service cannot determine whether the command ran. Such an
        /// execution is never retried automatically.
        outcome_unknown: bool,
    },
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
    /// True once output hit its retention cap and older events were dropped.
    pub output_truncated: bool,
    pub created_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub finished_at: Option<SystemTime>,
}

/// Parameters for running a command.
///
/// Construct with `Execute::builder()`. The argument vector is required;
/// environment overrides default to empty and all optional fields default to
/// `None`. No shell, standard input, deadline, or idempotency key is added
/// implicitly. Omitting `cwd` uses the sandbox's default working directory.
///
/// ```
/// use reprise_api::Execute;
/// use std::time::Duration;
///
/// let command = Execute::builder()
///     .argv(["echo"])
///     .arg("hello")
///     .env("MODE", "test")
///     .envs([("LANG", "C"), ("TZ", "UTC")])
///     .timeout(Duration::from_secs(30))
///     .build();
///
/// assert_eq!(command.argv, ["echo", "hello"]);
/// assert_eq!(command.env["MODE"], "test");
/// assert_eq!(command.cwd, None);
/// ```
/// ```compile_fail
/// use reprise_api::Execute;
///
/// // An argument vector must be supplied before building.
/// let command = Execute::builder().build();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct Execute {
    /// A typed argument vector. There is no implicit shell; a caller that wants
    /// shell semantics sends `["sh", "-lc", script]`.
    #[builder(
        setter(transform = |argv: impl IntoIterator<Item = impl Into<String>>| argv.into_iter().map(Into::into).collect()),
        mutators(
            /// Append one argument after supplying the initial argument vector.
            pub fn arg(&mut self, arg: impl Into<String>) {
                self.argv.push(arg.into());
            }
            /// Append arguments in iterator order.
            pub fn args(&mut self, args: impl IntoIterator<Item = impl Into<String>>) {
                self.argv.extend(args.into_iter().map(Into::into));
            }
        )
    )]
    pub argv: Vec<String>,

    /// Workspace-relative working directory. Defaults to `None`.
    #[builder(default, setter(strip_option, into))]
    pub cwd: Option<String>,

    /// Explicit environment overrides, layered over the template defaults.
    /// Defaults to an empty map.
    #[builder(via_mutators, setter(suffix = "_map"), mutators(
        /// Insert an override, replacing any previous value for this key.
        pub fn env(&mut self, key: impl Into<String>, value: impl Into<String>) {
            self.env.insert(key.into(), value.into());
        }
        /// Merge overrides. Later entries win for duplicate keys.
        pub fn envs(&mut self, pairs: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) {
            self.env.extend(pairs.into_iter().map(|(key, value)| (key.into(), value.into())));
        }
    ))]
    pub env: BTreeMap<String, String>,

    /// Optional, bounded standard input. Interactive terminals are out of
    /// scope. Defaults to `None`.
    #[builder(default, setter(strip_option, into))]
    pub stdin: Option<Vec<u8>>,

    /// Ends the command with [`ExecutionState::TimedOut`] after TERM and KILL.
    /// Defaults to `None` (no command deadline).
    #[builder(default, setter(strip_option, into))]
    pub timeout: Option<Duration>,

    /// Optional deduplication key. Defaults to `None`.
    #[builder(default, setter(strip_option, into))]
    pub idempotency_key: Option<IdempotencyKey>,
}

/// One item in an execution's output log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionEvent {
    /// Per-execution, strictly increasing and gap-free. It is the cursor a
    /// caller resumes from.
    pub sequence: u64,
    pub kind: ExecutionEventKind,
}

/// What an [`ExecutionEvent`] reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionEventKind {
    Output {
        channel: Channel,
        chunk: Vec<u8>,
    },
    /// A state change. The terminal status is also readable from the
    /// [`Execution`] itself.
    Status(ExecutionState),
    /// Output retention dropped older events; reading before this point is no
    /// longer possible.
    Truncated,
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
