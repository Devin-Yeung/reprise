//! Nix runtime closures represented as a read-only bind-mount layer.
use std::fs;
use std::path::{Path, PathBuf};

use crate::{Bind, Error, Layer};

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

    /// Store objects in manifest order.
    pub fn store_paths(&self) -> &[PathBuf] {
        &self.store_paths
    }
}

/// Exposes the closure as read-only binds of each store object at its own path.
///
/// Objects are bound individually, so the container sees only the closure and
/// not the host's whole store. Stack the layer on a [`Rootfs`](crate::Rootfs)
/// and call `prepare` to create the mount targets.
impl From<&NixClosure> for Layer {
    fn from(closure: &NixClosure) -> Self {
        Layer::binds(
            closure
                .store_paths()
                .iter()
                .map(|path| Bind::read_only(path, path)),
        )
    }
}
