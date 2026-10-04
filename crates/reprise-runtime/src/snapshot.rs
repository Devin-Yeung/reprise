use std::path::{Path, PathBuf};

use crate::error::Error;

/// A saved instance, which [`Runtime::restore`](crate::Runtime::restore) can
/// restore any number of times.
///
/// A snapshot is a directory holding what runsc wrote, the bundle the instance
/// was cold-booted with, and the runsc version, platform and checkpoint options
/// it was taken with. The directory appears only once complete.
#[derive(Debug)]
pub struct Snapshot {
    path: PathBuf,
}

impl Snapshot {
    /// Opens a snapshot that [`Instance::checkpoint`](crate::Instance::checkpoint)
    /// wrote, possibly in an earlier process.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        todo!()
    }

    /// The snapshot's directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Deletes the snapshot's directory.
    pub fn delete(self) -> Result<(), Error> {
        todo!()
    }
}
