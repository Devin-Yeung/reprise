use std::fmt;

use bollard::errors::Error as DockerError;

use crate::archive::ArchiveError;
use crate::image::ImageId;

/// Failure connecting to Docker or preparing a saved image.
#[derive(Debug)]
pub enum PrepareError {
    /// The connect deadline elapsed.
    ConnectTimeout,
    /// The Unix socket could not be opened, or the Engine version could not be negotiated.
    Connect(DockerError),
    /// The preparation deadline elapsed.
    ///
    /// A blocking archive read or an engine-side load may still be running.
    PrepareTimeout,
    /// The path is not one readable single-image Docker save.
    Archive(ArchiveError),
    /// Inspect or upload failed for a reason other than a missing image or a rejected load.
    Engine(DockerError),
    /// The engine accepted the load request and then reported an error.
    LoadRejected { message: String },
    /// After upload, inspect did not return this archive's image ID.
    IdentityMismatch { expected: ImageId },
}

impl fmt::Display for PrepareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectTimeout => f.write_str("timed out connecting to Docker"),
            Self::Connect(error) => write!(f, "connect to Docker: {error}"),
            Self::PrepareTimeout => f.write_str("timed out preparing image"),
            Self::Archive(error) => write!(f, "{error}"),
            Self::Engine(error) => write!(f, "Docker engine: {error}"),
            Self::LoadRejected { message } => {
                write!(f, "Docker rejected image load: {message}")
            }
            Self::IdentityMismatch { expected } => {
                write!(f, "loaded image does not match {expected}")
            }
        }
    }
}

impl std::error::Error for PrepareError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Connect(source) | Self::Engine(source) => Some(source),
            Self::Archive(source) => Some(source),
            Self::ConnectTimeout
            | Self::PrepareTimeout
            | Self::LoadRejected { .. }
            | Self::IdentityMismatch { .. } => None,
        }
    }
}

impl From<ArchiveError> for PrepareError {
    fn from(error: ArchiveError) -> Self {
        Self::Archive(error)
    }
}
