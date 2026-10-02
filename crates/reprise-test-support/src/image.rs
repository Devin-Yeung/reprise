use bollard::models::ImageInspect;

/// Whether this preparation call needed to upload the archive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImagePreparation {
    /// The image was already present.
    Cached,
    /// This call uploaded the archive.
    Loaded,
}

/// An image whose ID and platform match the archive and Docker host.
///
/// `image.id` contains the full `sha256:` digest of the archive's exact config
/// bytes. Use it to configure the daemon on the same Docker endpoint.
#[derive(Clone, Debug)]
pub struct PreparedImage {
    /// Docker's inspected response, verified against the local archive.
    pub image: ImageInspect,
    pub preparation: ImagePreparation,
}
