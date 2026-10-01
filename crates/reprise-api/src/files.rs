//! The workspace file surface.
//!
//! The workspace is the only managed file surface. Paths resolve inside it;
//! host paths and mounts are not caller-controlled.

/// A content digest of a file version.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Digest(String);

/// Read a workspace file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadFile {
    /// Workspace-relative path.
    pub path: String,
    /// Refuse to return more than this many bytes.
    pub max_bytes: Option<u64>,
}

/// Write a workspace file.
///
/// The write is staged and renamed into place. When `expected_digest` is set,
/// the write applies only if the file still matches it, otherwise it is a
/// conflict rather than an overwrite. This is compare-and-swap against the
/// version the caller last saw; it is not a CAS against unmanaged background
/// writers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteFile {
    /// Workspace-relative path.
    pub path: String,
    pub data: Vec<u8>,
    pub expected_digest: Option<Digest>,
}

/// The contents of a workspace file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileContent {
    pub path: String,
    pub data: Vec<u8>,
    pub digest: Digest,
    /// True when the file exceeded `max_bytes` and was cut short.
    pub truncated: bool,
}

/// The result of writing a workspace file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileVersion {
    pub path: String,
    pub digest: Digest,
    pub size: u64,
}
