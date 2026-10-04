//! Nix runtime closures and their materialization into a root filesystem.

use std::fs;
use std::path::{Path, PathBuf};

use crate::copy::ClosureCopier;
use crate::{Error, Rootfs};

/// Store objects listed by a Nix closure artifact for a workload.
///
/// The artifact producer supplies complete, absolute store paths. The objects
/// must remain unchanged during materialization.
#[derive(Debug)]
pub struct NixClosure {
    store_paths: Vec<PathBuf>,
}

impl NixClosure {
    /// Reads the given `store-paths` file as absolute store paths, one per line.
    pub fn load(manifest_path: &Path) -> Result<Self, Error> {
        let manifest = fs::read_to_string(manifest_path)?;
        let store_paths = manifest.lines().map(PathBuf::from).collect();
        Ok(Self { store_paths })
    }

    /// Store objects in manifest order.
    pub fn store_paths(&self) -> &[PathBuf] {
        &self.store_paths
    }

    /// Copies the closure into a new directory, preserving its `/nix/store` layout.
    ///
    /// - The destination's parent must exist.
    /// - Preserved: file contents, symbolic links, hard links, and file/directory
    ///   permission bits. Other file types are rejected.
    /// - Not preserved: owners, timestamps and extended attributes.
    /// - A failure after directory creation leaves a partial destination for the
    ///   caller to remove before retrying.
    pub fn materialize(&self, destination: &Path) -> Result<Rootfs, Error> {
        self.ensure_destination_outside_closure(destination)?;

        fs::create_dir(destination)?;
        let rootfs_path = fs::canonicalize(destination)?;
        let destination_store = rootfs_path.join("nix/store");
        fs::create_dir_all(&destination_store)?;

        // One copier spans the closure so hard links across store objects retain
        // their identity, while every copied file is independent of the host store.
        let mut copier = ClosureCopier::default();
        for store_path in &self.store_paths {
            let name = store_path
                .file_name()
                .expect("store paths in the manifest name store objects");
            copier.copy(store_path, &destination_store.join(name))?;
        }
        Ok(Rootfs::prepared(rootfs_path))
    }

    /// Rejects a destination inside any closure object, before it is created.
    ///
    /// Copying into a source would mutate the closure and make recursive copying
    /// unbounded, so this must run before the destination exists.
    fn ensure_destination_outside_closure(&self, destination: &Path) -> Result<(), Error> {
        // Resolve the parent, not the destination: the destination is about to
        // be created and does not exist yet.
        let parent = destination
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let parent = fs::canonicalize(parent)?;
        for path in &self.store_paths {
            let metadata = fs::symlink_metadata(path)?;
            // A root object may itself be a dangling symlink. Only directories
            // can contain the destination; resolving links here would reject
            // valid contents that the copier deliberately preserves verbatim.
            if !metadata.is_dir() {
                continue;
            }
            let source = fs::canonicalize(path)?;
            if parent.starts_with(&source) {
                return Err(Error::DestinationInsideClosure(source));
            }
        }
        Ok(())
    }
}
