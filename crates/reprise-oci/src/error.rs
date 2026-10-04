use std::io;
use std::path::{Path, PathBuf};

/// A failure to load a closure or prepare its filesystem.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Copying into a source would modify the closure and recurse into itself.
    #[error("rootfs destination is inside closure object: {0}")]
    DestinationInsideClosure(PathBuf),
    /// Nix store contents are files, directories and symbolic links.
    #[error("unsupported closure file type: {0}")]
    UnsupportedFileType(PathBuf),
    /// A filesystem operation failed at the named path.
    #[error("{operation} {}: {source}", path.display())]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// Names the operation and path of a failed filesystem call.
pub(crate) trait IoResultExt<T> {
    fn context(self, operation: &'static str, path: &Path) -> Result<T, Error>;
}

impl<T> IoResultExt<T> for io::Result<T> {
    fn context(self, operation: &'static str, path: &Path) -> Result<T, Error> {
        self.map_err(|source| Error::Io {
            operation,
            path: path.to_owned(),
            source,
        })
    }
}
