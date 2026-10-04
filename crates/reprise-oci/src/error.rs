use std::io;
use std::path::PathBuf;

/// A failure to load a closure or prepare its filesystem.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Copying into a source would modify the closure and recurse into itself.
    #[error("rootfs destination is inside closure object: {0}")]
    DestinationInsideClosure(PathBuf),
    /// Nix store contents are files, directories and symbolic links.
    #[error("unsupported closure file type: {0}")]
    UnsupportedFileType(PathBuf),
    /// A filesystem operation failed.
    #[error(transparent)]
    Io(#[from] io::Error),
}
