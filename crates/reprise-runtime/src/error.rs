use std::path::PathBuf;
use std::process::ExitStatus;

use crate::runtime::Platform;

/// A failed runtime operation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Starting instances needs root.
    #[error("the runtime must run as root")]
    NotRoot,

    /// Another [`Runtime`](crate::Runtime) exists on this host.
    #[error("another runtime exists on this host")]
    Busy,

    /// The snapshot was rejected before runsc ran.
    #[error("incompatible snapshot: {0}")]
    Incompatible(Incompatibility),

    /// A command such as `runsc restore` exited unsuccessfully.
    #[error("`{command}` failed with {status}: {stderr}")]
    Command {
        command: String,
        status: ExitStatus,
        stderr: String,
    },

    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Why [`Runtime::restore`](crate::Runtime::restore) cannot restore a snapshot.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Incompatibility {
    #[error("written by runsc {snapshot}, but the runtime runs {runtime}")]
    RunscVersion { snapshot: String, runtime: String },

    #[error("taken on {snapshot:?}, but the runtime runs on {runtime:?}")]
    Platform {
        snapshot: Platform,
        runtime: Platform,
    },

    /// [`RestoreOptions::background`](crate::RestoreOptions::background) was
    /// requested for a compressed snapshot.
    #[error("background restore needs an uncompressed snapshot")]
    BackgroundCompressed,
}
