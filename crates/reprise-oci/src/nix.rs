//! Nix runtime closures represented as OCI read-only bind mounts.
use std::fs;
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
        let manifest = fs::read_to_string(manifest_path)?;
        let store_paths = manifest.lines().map(PathBuf::from).collect();
        Ok(Self { store_paths })
    }

    /// Creates or reuses a shared base directory without copying store objects.
    /// Prepares `nix/store` inside it; individual mount points and runtime
    /// directories remain the caller's responsibility.
    /// The caller owns the directory and keeps it available for its containers.
    pub fn to_rootfs(&self, root: impl AsRef<Path>) -> Result<NixBasedRootFS, Error> {
        fs::create_dir_all(root.as_ref().join("nix/store"))?;
        Ok(NixBasedRootFS {
            root: fs::canonicalize(root)?,
            closure: self.clone(),
        })
    }

    /// Store objects in manifest order.
    pub fn store_paths(&self) -> &[PathBuf] {
        &self.store_paths
    }
}

/// A shared base directory and the closure mounted into its container view.
///
/// Dropping this value does not remove the directory or its dependencies. The
/// caller provides writable locations and runtime mounts in the complete spec.
pub struct NixBasedRootFS {
    root: PathBuf,
    closure: NixClosure,
}

impl NixBasedRootFS {
    /// Read-only base root using the absolute path of the prepared directory.
    pub fn oci_root(&self) -> Root {
        let mut root = Root::default();
        root.set_path(self.root.clone()).set_readonly(Some(true));
        root
    }

    /// Read-only store-object mounts, in closure manifest order.
    pub fn oci_mounts(&self) -> Vec<Mount> {
        self.closure
            .store_paths()
            .iter()
            .map(|path| {
                let mut mount = Mount::default();
                mount
                    .set_source(Some(path.clone()))
                    .set_destination(path.clone())
                    .set_typ(Some("bind".into()))
                    .set_options(Some(vec!["bind".into(), "ro".into()]));
                mount
            })
            .collect()
    }
}
