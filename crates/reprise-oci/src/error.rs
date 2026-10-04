use std::io;

/// A failure to read a closure artifact.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
}
