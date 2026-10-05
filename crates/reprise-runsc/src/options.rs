use std::os::fd::OwnedFd;
use std::path::PathBuf;

use typed_builder::TypedBuilder;

/// Prepared OCI bundle and launch-time stdio, shared by create and run commands.
/// Console/TTY allocation is outside this interface; `process.terminal` must be false.
///
/// ```
/// use reprise_runsc::{CreateOptions, ContainerIo, OutputTarget};
///
/// let create = CreateOptions::builder()
///     .bundle("/var/lib/reprise/bundle")
///     .pid_file("/var/lib/reprise/container.pid")
///     .io(ContainerIo::builder().stderr(OutputTarget::Null).build())
///     .build();
/// assert_eq!(create.bundle, std::path::Path::new("/var/lib/reprise/bundle"));
/// assert!(matches!(create.io.stdout, OutputTarget::Inherit));
/// ```
///
/// The bundle must be supplied before building:
///
/// ```compile_fail
/// use reprise_runsc::CreateOptions;
/// let create = CreateOptions::builder().build();
/// ```
#[derive(Debug, TypedBuilder)]
pub struct CreateOptions {
    #[builder(setter(into))]
    pub bundle: PathBuf,
    /// Optional host file receiving the sandbox PID, not an application PID.
    #[builder(default, setter(strip_option, into))]
    pub pid_file: Option<PathBuf>,
    #[builder(default)]
    pub io: ContainerIo,
}

/// Output destinations established when creating a container. Input is `/dev/null`.
///
/// A descriptor is deliberately one-shot. The caller transfers ownership to
/// these launch options, and must release the options after launch when EOF is
/// meaningful to an output collector. Sandbox descendants can retain a copy of
/// the descriptor, so EOF only ends an output stream; it does not prove that a
/// container has completed.
#[derive(Debug, Default, TypedBuilder)]
#[builder(field_defaults(default))]
pub struct ContainerIo {
    pub stdout: OutputTarget,
    pub stderr: OutputTarget,
}

#[derive(Debug, Default)]
pub enum OutputTarget {
    #[default]
    Inherit,
    Null,
    /// Opens a host file for append, creating it if missing without truncation.
    /// The caller prepares its parent directory and owns retention of the file.
    File(PathBuf),
    /// Transfers an already-open host descriptor to the launch configuration.
    ///
    /// `run_detached` duplicates it for runsc because launch options are
    /// borrowed. Drop the options after a successful launch to close this
    /// original descriptor and allow readers to observe EOF when runsc and the
    /// sandbox have closed their copies.
    Fd(OwnedFd),
}

/// Execution checkpoint options; these do not capture arbitrary external volumes.
///
/// ```
/// use reprise_runsc::{CheckpointOptions, Compression};
///
/// let checkpoint = CheckpointOptions::builder()
///     .image_path("/var/lib/reprise/checkpoint")
///     .compression(Compression::None)
///     .build();
/// assert!(!checkpoint.leave_running);
/// ```
///
/// ```compile_fail
/// use reprise_runsc::CheckpointOptions;
/// let checkpoint = CheckpointOptions::builder().build();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, TypedBuilder)]
pub struct CheckpointOptions {
    #[builder(setter(into))]
    pub image_path: PathBuf,
    #[builder(default)]
    pub leave_running: bool,
    /// `None` delegates to the installed runsc's compression default.
    #[builder(default, setter(strip_option))]
    pub compression: Option<Compression>,
    #[builder(default)]
    pub exclude_committed_zero_pages: bool,
    /// Writes checkpoint pages with O_DIRECT, bypassing the host page cache.
    #[builder(default)]
    pub direct_io: bool,
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
///
/// ```
/// use reprise_runsc::{CreateOptions, DeleteOptions, RestoreOptions};
///
/// let restore = RestoreOptions::builder()
///     .create(CreateOptions::builder().bundle("/var/lib/reprise/bundle").build())
///     .image_path("/var/lib/reprise/checkpoint")
///     .background(true)
///     .build();
/// let cleanup = DeleteOptions::builder().force(true).build();
/// assert!(restore.background && cleanup.force);
/// ```
///
/// Both launch configuration and image path are required:
///
/// ```compile_fail
/// use reprise_runsc::RestoreOptions;
/// let restore = RestoreOptions::builder().image_path("/checkpoint").build();
/// ```
///
/// ```compile_fail
/// use reprise_runsc::{CreateOptions, RestoreOptions};
/// let create = CreateOptions::builder().bundle("/bundle").build();
/// let restore = RestoreOptions::builder().create(create).build();
/// ```
#[derive(Debug, TypedBuilder)]
pub struct RestoreOptions {
    /// Supplies the bundle and launch stdio if runsc needs to create a container.
    pub create: CreateOptions,
    #[builder(setter(into))]
    pub image_path: PathBuf,
    /// Requests background image loading; effective only for uncompressed images.
    /// A compressed image can cause runsc to ignore this flag.
    #[builder(default)]
    pub background: bool,
    #[builder(default)]
    pub direct_io: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, TypedBuilder)]
#[builder(field_defaults(default))]
pub struct DeleteOptions {
    pub force: bool,
}
