use std::fs::File;
use std::path::{Path, PathBuf};

use bollard::models::Platform;

use crate::PreparationError;

/// An open archive retained for upload with its config-derived identity.
#[expect(
    dead_code,
    reason = "archive state is not wired to the environment yet"
)]
pub(crate) struct ValidatedArchive {
    pub(crate) path: PathBuf,
    pub(crate) file: File,
    pub(crate) image_id: String,
    pub(crate) platform: Platform,
}

/// Retain the opened file so validation and upload use the same artifact.
#[expect(dead_code, reason = "archive validation is not wired yet")]
pub(crate) fn validate_archive(_path: &Path) -> Result<ValidatedArchive, PreparationError> {
    todo!("open archive, read_config, inspect_config and retain the opened file")
}

/// Read the manifest's config bytes without reserializing JSON.
#[expect(dead_code, reason = "archive parsing is not wired yet")]
fn read_config(_file: &mut File) -> Result<Vec<u8>, PreparationError> {
    todo!("read manifest.json and its referenced configuration bytes from tar")
}

/// Hash the original config bytes: Docker IDs hash the config, not the tar.
#[expect(dead_code, reason = "config identity derivation is not wired yet")]
fn inspect_config(_bytes: &[u8]) -> Result<(String, Platform), PreparationError> {
    todo!("hash original configuration bytes and parse OS/architecture")
}
