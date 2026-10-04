use std::path::PathBuf;

/// Prepared OCI bundle and launch-time stdio, shared by create and run commands.
/// Console/TTY allocation is outside this interface; `process.terminal` must be false.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateOptions {
    pub bundle: PathBuf,
    /// Optional host file receiving the sandbox PID, not an application PID.
    pub pid_file: Option<PathBuf>,
    pub io: ContainerIo,
}

impl CreateOptions {
    /// Uses inherited stdout/stderr and closed input for a noninteractive workload.
    pub fn new(bundle: impl Into<PathBuf>) -> Self {
        Self {
            bundle: bundle.into(),
            pid_file: None,
            io: ContainerIo::default(),
        }
    }
}

/// Output destinations established when creating a container. Input is `/dev/null`.
/// Pipes are intentionally omitted: sandbox children can keep them open after
/// the CLI exits, preventing an output collector from reaching EOF.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContainerIo {
    pub stdout: OutputTarget,
    pub stderr: OutputTarget,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum OutputTarget {
    #[default]
    Inherit,
    Null,
    /// Opens a host file for append, creating it if missing without truncation.
    /// The caller prepares its parent directory and owns retention of the file.
    File(PathBuf),
}

/// Execution checkpoint options; these do not capture arbitrary external volumes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointOptions {
    pub image_path: PathBuf,
    pub leave_running: bool,
    /// `None` delegates to the installed runsc's compression default.
    pub compression: Option<Compression>,
    pub exclude_committed_zero_pages: bool,
    /// Writes checkpoint pages with O_DIRECT, bypassing the host page cache.
    pub direct_io: bool,
}

impl CheckpointOptions {
    /// Keeps runsc's compression default and stops the container after checkpoint.
    pub fn new(image_path: impl Into<PathBuf>) -> Self {
        Self {
            image_path: image_path.into(),
            leave_running: false,
            compression: None,
            exclude_committed_zero_pages: false,
            direct_io: false,
        }
    }
}

/// Compression choices supported by runsc's execution checkpoint CLI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Compression {
    None,
    FlateBestSpeed,
}

/// Detached restore from an execution checkpoint.
/// The caller retains the image and unchanged dependencies, and checks runsc,
/// platform, CPU, and bundle compatibility before invoking restore.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreOptions {
    /// Supplies the bundle and launch stdio if runsc needs to create a container.
    pub create: CreateOptions,
    pub image_path: PathBuf,
    /// Requests background image loading; effective only for uncompressed images.
    /// A compressed image can cause runsc to ignore this flag.
    pub background: bool,
    pub direct_io: bool,
}

impl RestoreOptions {
    pub fn new(create: CreateOptions, image_path: impl Into<PathBuf>) -> Self {
        Self {
            create,
            image_path: image_path.into(),
            background: false,
            direct_io: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeleteOptions {
    pub force: bool,
}
