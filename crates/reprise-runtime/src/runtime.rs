use std::path::PathBuf;

use crate::error::Error;
use crate::instance::Instance;
use crate::snapshot::Snapshot;

/// Configuration for [`Runtime::new`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeConfig {
    /// The `runsc` binary. A snapshot restores only with the runsc version that
    /// wrote it.
    pub runsc: PathBuf,
    /// Holds bundles, runsc state and snapshots. Created if missing.
    pub state_dir: PathBuf,
    pub platform: Platform,
}

/// How the gVisor Sentry intercepts the application's system calls
/// (`runsc --platform`). A snapshot restores only on the platform it was taken
/// on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Platform {
    /// Seccomp traps; needs no hardware virtualization. runsc's default.
    #[default]
    Systrap,
    /// Hardware virtualization through `/dev/kvm`, nested when the host is
    /// itself a VM such as WSL2.
    Kvm,
}

/// What a cold boot runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workload {
    /// A self-contained root filesystem: nothing in it may link outside it, for
    /// example into `/nix/store`. It stays read-only; the sandbox's file writes
    /// are kept in memory, so snapshots include them.
    pub rootfs: PathBuf,
    /// The process's argv. `args[0]` is an absolute path inside `rootfs`.
    pub args: Vec<String>,
    /// The process's entire environment, as `KEY=VALUE` entries.
    pub env: Vec<String>,
}

/// `runsc restore` flags under study. The default is runsc's: the baseline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RestoreOptions {
    /// Return before all memory is loaded and load the rest while the workload
    /// runs (`--background`). Needs an uncompressed snapshot.
    pub background: bool,
    /// Read the snapshot with `O_DIRECT`, bypassing the host page cache
    /// (`--direct`).
    pub direct_io: bool,
}

/// Starts instances, one at a time.
///
/// [`cold_boot`](Self::cold_boot) and [`restore`](Self::restore) borrow the
/// runtime until the returned [`Instance`] ends, because every instance uses the
/// same network namespace and addresses.
#[derive(Debug)]
pub struct Runtime {
    config: RuntimeConfig,
    runsc_version: String,
}

impl Runtime {
    /// Takes over `state_dir` and deletes the instances an earlier process left
    /// there. Snapshots are kept.
    ///
    /// Fails with [`Error::NotRoot`] unless running as root, and with
    /// [`Error::Busy`] while another `Runtime` exists on this host.
    pub fn new(config: RuntimeConfig) -> Result<Self, Error> {
        todo!()
    }

    /// The version `runsc` reported to [`new`](Self::new).
    pub fn runsc_version(&self) -> &str {
        &self.runsc_version
    }

    /// Starts `workload` from the beginning.
    ///
    /// Returns once `runsc start` exits; the workload may not answer yet.
    pub fn cold_boot(&mut self, workload: &Workload) -> Result<Instance<'_>, Error> {
        todo!()
    }

    /// Starts an instance that continues from `snapshot`.
    ///
    /// Returns once `runsc restore` exits; the workload may not answer yet. A
    /// snapshot that does not match this runtime or `options` fails with
    /// [`Error::Incompatible`] before runsc runs. A failed restore is an error,
    /// never a cold boot.
    ///
    /// A snapshot can be restored any number of times. The instance borrows
    /// `snapshot` because a background restore keeps reading it after this
    /// returns.
    pub fn restore<'a>(
        &'a mut self,
        snapshot: &'a Snapshot,
        options: &RestoreOptions,
    ) -> Result<Instance<'a>, Error> {
        todo!()
    }
}
