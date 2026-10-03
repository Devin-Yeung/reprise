//! Prepare saved Docker images for tests on an explicit Unix socket.
//!
//! Connect with [`DockerEndpoint::connect`], then prepare an archive with
//! [`DockerEndpoint::ensure_image`]. Use [`PreparedImage::id`] to configure the
//! daemon on the same endpoint.
//!
//! Parallel callers may share an endpoint and image. Preparation never removes
//! shared images; concurrent cache misses may each upload the same archive.
//! Connection and preparation return [`PrepareError`].

mod archive;
mod environment;
mod error;
mod image;

pub use archive::ArchiveError;
pub use environment::DockerEndpoint;
pub use error::PrepareError;
pub use image::{ImageId, ImagePreparation, PreparedImage};
