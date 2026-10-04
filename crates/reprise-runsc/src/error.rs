use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitStatus;

/// A failed runsc operation, retaining the exact invocation for diagnostics.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("runsc {invocation:?}: {source}")]
    Io {
        invocation: Box<Invocation>,
        #[source]
        source: std::io::Error,
    },
    #[error("runsc {invocation:?} exited with {status}")]
    Command {
        invocation: Box<Invocation>,
        status: ExitStatus,
        /// Present for captured control-command output; absent for redirected
        /// launch stdio, whose diagnostics remain at the caller's destinations.
        stdout: Option<Vec<u8>>,
        stderr: Option<Vec<u8>>,
    },
    /// A successful CLI invocation returned an unusable version/state/list/wait
    /// response. Original bytes are retained instead of decoding them lossily.
    #[error("invalid runsc {invocation:?} response: {reason}")]
    InvalidOutput {
        invocation: Box<Invocation>,
        reason: String,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    },
}

/// Actual executable and argv, not a shell-escaped display string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invocation {
    pub executable: PathBuf,
    pub operation: Operation,
    pub args: Vec<OsString>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operation {
    Version,
    Create,
    Start,
    Run,
    Checkpoint,
    Restore,
    State,
    List,
    Wait,
    Delete,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid container ID: {value:?}")]
pub struct InvalidContainerId {
    pub value: String,
}
