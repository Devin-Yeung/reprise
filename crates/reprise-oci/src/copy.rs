//! Materializing a Nix closure, preserving the subset of the filesystem that
//! Nix store contents use: files, directories and symbolic links.

use std::collections::HashMap;
use std::fs::{self, Metadata};
use std::os::unix::fs::{MetadataExt, symlink};
use std::path::{Path, PathBuf};

use crate::Error;
use crate::error::IoResultExt;

/// Copies the objects of one closure, retaining hard links between them.
///
/// - The source tree must stay unchanged for the copier's lifetime; Nix store
///   immutability guarantees this.
/// - Hard links are remapped within the copy, so no materialized file links
///   back to the host store.
#[derive(Default)]
pub(crate) struct ClosureCopier {
    hard_links: HardLinks,
}

impl ClosureCopier {
    /// Copies the entry at `source` to exactly `destination`.
    ///
    /// - `destination` is the new path itself, not a directory to copy into, and
    ///   its parent must already exist.
    /// - Symbolic links are preserved verbatim, including dangling targets.
    /// - A file whose source inode was already copied becomes a hard link to that
    ///   earlier copy.
    /// - Other file types are rejected.
    pub(crate) fn copy(&mut self, source: &Path, destination: &Path) -> Result<(), Error> {
        // Never follow links: absolute links address the sandbox's filesystem,
        // and dangling links are valid closure contents too.
        let metadata = fs::symlink_metadata(source).context("inspect closure entry", source)?;
        let file_type = metadata.file_type();

        match file_type {
            kind if kind.is_symlink() => copy_symlink(source, destination),
            kind if kind.is_dir() => self.copy_directory(source, destination, &metadata),
            kind if kind.is_file() => {
                self.hard_links
                    .copy_or_relink(source, destination, &metadata)
            }
            // FIFOs could block a byte copier; sockets and devices are runtime
            // resources, not Nix closure contents. Fail rather than omit them.
            _ => Err(Error::UnsupportedFileType(source.to_owned())),
        }
    }

    fn copy_directory(
        &mut self,
        source: &Path,
        destination: &Path,
        metadata: &Metadata,
    ) -> Result<(), Error> {
        fs::create_dir(destination).context("create directory", destination)?;

        for entry in fs::read_dir(source).context("read directory", source)? {
            let entry = entry.context("read directory entry", source)?;
            self.copy(&entry.path(), &destination.join(entry.file_name()))?;
        }
        // Store directories are often read-only. Set their final permissions
        // only after children exist, so an unprivileged caller can populate them.
        fs::set_permissions(destination, metadata.permissions())
            .context("set directory permissions", destination)
    }
}

fn copy_symlink(source: &Path, destination: &Path) -> Result<(), Error> {
    let target = fs::read_link(source).context("read symbolic link", source)?;
    symlink(target, destination).context("create symbolic link", destination)
}

/// Remembers where each already-copied source inode landed, so a later hard link
/// to the same inode can re-link to that copy instead of duplicating its bytes.
#[derive(Default)]
struct HardLinks {
    first_copy: HashMap<(u64, u64), PathBuf>,
}

impl HardLinks {
    fn copy_or_relink(
        &mut self,
        source: &Path,
        destination: &Path,
        metadata: &Metadata,
    ) -> Result<(), Error> {
        let identity = (metadata.dev(), metadata.ino());
        if let Some(existing) = self.first_copy.get(&identity) {
            return fs::hard_link(existing, destination).context("create hard link", destination);
        }
        fs::copy(source, destination).context("copy file", destination)?;
        // A file with a single link cannot be referenced again, so there is
        // nothing to remember.
        if metadata.nlink() > 1 {
            self.first_copy.insert(identity, destination.to_owned());
        }
        Ok(())
    }
}
