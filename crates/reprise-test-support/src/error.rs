use std::path::PathBuf;

/// The activity in which preparation failed or exhausted its time budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparationStage {
    Connect,
    ReadArchive,
    InspectHost,
    InspectImage,
    LoadImage,
    VerifyImage,
}

/// Image preparation failures with Docker and I/O errors preserved.
#[derive(Debug)]
pub enum PreparationError {
    InvalidInput { reason: String },
    ArchiveIo {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidArchive { path: PathBuf, reason: String },
    /// The archive was detectably modified during preparation.
    ArchiveChanged { path: PathBuf },
    /// Unsupported or mismatched platforms, described as OS/architecture/variant.
    PlatformMismatch { archive: String, host: String },
    Docker {
        stage: PreparationStage,
        source: bollard::errors::Error,
    },
    /// Docker's image facts were missing, malformed or inconsistent with the archive.
    VerificationFailed {
        expected_image_id: String,
        reason: String,
    },
    Timeout { stage: PreparationStage },
}
