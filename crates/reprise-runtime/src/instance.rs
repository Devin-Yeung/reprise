use std::net::IpAddr;

use crate::error::Error;
use crate::runtime::Runtime;
use crate::snapshot::Snapshot;

/// `runsc checkpoint` flags under study. The default is runsc's: the baseline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CheckpointOptions {
    /// Leave all-zero memory pages out of the snapshot
    /// (`--exclude-committed-zero-pages`).
    pub exclude_zero_pages: bool,
    /// Compress memory (`--compression=flate-best-speed`). A compressed snapshot
    /// cannot be restored in the background.
    pub compress: bool,
    /// Write memory with `O_DIRECT`, bypassing the host page cache (`--direct`).
    pub direct_io: bool,
}

/// A running sandbox, from [`Runtime::cold_boot`] or [`Runtime::restore`].
///
/// End it with [`checkpoint`](Self::checkpoint) or [`destroy`](Self::destroy).
/// Dropping it destroys it too, ignoring errors.
#[derive(Debug)]
pub struct Instance<'a> {
    runtime: &'a mut Runtime,
}

impl Instance<'_> {
    /// The sandbox's address, reachable from the host; the workload listens on
    /// it or on `0.0.0.0`. Every instance has the same address, so a restored
    /// workload keeps the one it was checkpointed with.
    pub fn address(&self) -> IpAddr {
        todo!()
    }

    /// Saves the instance into a new snapshot under the runtime's `state_dir`.
    /// The instance ends whether or not this succeeds.
    pub fn checkpoint(self, options: &CheckpointOptions) -> Result<Snapshot, Error> {
        todo!()
    }

    /// Stops the sandbox and deletes its runsc state, bundle and network.
    pub fn destroy(self) -> Result<(), Error> {
        todo!()
    }
}

impl Drop for Instance<'_> {
    fn drop(&mut self) {
        todo!()
    }
}
