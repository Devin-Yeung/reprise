//! Prepare saved Docker images for tests on an explicit Unix socket.
//!
//! Connect with [`DockerTestEnvironment::connect`], then prepare an archive with
//! [`DockerTestEnvironment::ensure_image`]. Use the returned image ID to configure
//! the daemon on the same endpoint.
//!
//! Parallel callers may share an endpoint and image. Preparation never removes
//! shared images; concurrent cache misses may each upload the same archive.

mod archive;
mod environment;
mod image;

pub use environment::DockerTestEnvironment;
pub use image::{ImagePreparation, PreparedImage};
