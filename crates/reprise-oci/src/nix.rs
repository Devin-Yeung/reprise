//! Nix runtime closures represented as OCI read-only bind mounts.
use std::path::{Path, PathBuf};

use oci_spec::runtime::{Mount, Root};

use crate::Error;

/// Store objects listed by a complete local Nix runtime closure artifact.
///
/// The artifact producer supplies complete absolute store paths and the target
/// architecture. The caller keeps these objects available while containers or
/// snapshots use them; loading only reads metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NixClosure {
    store_paths: Vec<PathBuf>,
}

impl NixClosure {
    /// Reads a `store-paths` file and describes its objects as read-only mounts.
    pub fn load(manifest_path: impl AsRef<Path>) -> Result<Self, Error> {
        todo!()
    }

    pub fn to_rootfs(&self) -> Result<NixBasedRootFS, Error> {
        todo!()
    }

    /// Store objects in manifest order.
    pub fn store_paths(&self) -> &[PathBuf] {
        &self.store_paths
    }
}

pub struct NixBasedRootFS {
    root: PathBuf,
    closure: NixClosure,
}

impl NixBasedRootFS {
    /// Read-only base root, defaulting to `rootfs` relative to the bundle.
    /// The caller creates the directory or changes the path in its own spec.
    pub fn oci_root(&self) -> Root {
        todo!()
    }

    /// Read-only store-object mounts, in closure manifest order.
    pub fn oci_mounts(&self) -> Vec<Mount> {
        todo!()
    }
}
