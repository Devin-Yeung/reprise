use oci_spec::runtime::Root;
use std::path::{Path, PathBuf};

/// A successfully prepared root filesystem, identified by its absolute host path.
///
/// - The caller owns its lifetime and must keep it available to the runtime.
/// - Dropping the handle neither freezes nor deletes the directory.
#[derive(Debug)]
pub struct Rootfs {
    pub(crate) path: PathBuf,
}

impl Rootfs {
    pub fn oci_spec(&self) -> Root {
        let mut root = Root::default();
        root.set_path(self.path.clone()).set_readonly(Some(true));
        root
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
