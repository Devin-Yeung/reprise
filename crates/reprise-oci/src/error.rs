use std::io;

/// A failure to read a closure artifact.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("unsupported runtime artifact format version {version}")]
    UnsupportedRuntimeArtifactFormat { version: u32 },
    #[error("runtime artifact command profile {profile:?} is absent from its closure")]
    CommandProfileOutsideClosure { profile: std::path::PathBuf },
    #[error("runtime artifact command profile has no bin directory: {profile:?}")]
    MissingCommandProfileBin { profile: std::path::PathBuf },
}
