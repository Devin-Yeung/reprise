use std::path::{Path, PathBuf};

/// A successfully prepared root filesystem, identified by its absolute host path.
///
/// - The caller owns its lifetime and must keep it available to the runtime.
/// - Dropping the handle neither freezes nor deletes the directory.
#[derive(Debug)]
pub struct Rootfs {
    path: PathBuf,
}

impl Rootfs {
    pub(crate) fn prepared(path: PathBuf) -> Self {
        Self { path }
    }

    /// Absolute host path to the prepared filesystem.
    pub fn as_path(&self) -> &Path {
        &self.path
    }

    /// Transfers the host path to a caller such as a runtime workload descriptor.
    pub fn into_path(self) -> PathBuf {
        self.path
    }
}
