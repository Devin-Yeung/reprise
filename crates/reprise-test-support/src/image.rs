use std::fmt;

use sha2::{Digest, Sha256};

/// Docker image ID: `sha256:` plus the digest of a saved image's exact config bytes.
///
/// This is not the checksum of the save tar, and not a registry manifest digest.
/// Whitespace in the config changes the ID.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImageId(String);

impl ImageId {
    pub(crate) fn from_config_bytes(bytes: &[u8]) -> Self {
        Self(format!("sha256:{:x}", Sha256::digest(bytes)))
    }

    /// The `sha256:…` string to pass to the daemon on the same Docker endpoint.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ImageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Whether this preparation call uploaded the archive.
///
/// Concurrent callers that both miss the cache can each observe [`Self::Loaded`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImagePreparation {
    /// Inspect already found the archive's image ID.
    Cached,
    /// This call streamed the archive to the engine.
    Loaded,
}

/// A saved image made present on the endpoint that prepared it.
///
/// `id` comes from the archive config bytes. Use it to configure the daemon on
/// this same endpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedImage {
    pub id: ImageId,
    pub preparation: ImagePreparation,
}
