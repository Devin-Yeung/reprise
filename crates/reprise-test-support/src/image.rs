use bollard::models::ImageInspect;

/// Whether this preparation call needed to upload the archive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImagePreparation {
    /// The image was already present.
    Cached,
    /// This call uploaded the archive.
    Loaded,
}

/// A test image inspected by its immutable archive ID.
///
/// `image.id` contains the full `sha256:` digest of the archive's exact config
/// bytes. Use it to configure the daemon on the same Docker endpoint.
#[derive(Clone, Debug)]
pub struct PreparedImage {
    /// Docker's response to an inspection by image ID.
    pub image: ImageInspect,
    pub preparation: ImagePreparation,
}
