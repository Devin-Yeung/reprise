//! Prepare saved Docker images for tests on an explicit Unix socket.
//!
//! Connect with [`DockerTestEnvironment::connect`], then prepare an archive with
//! [`DockerTestEnvironment::ensure_image`]. Use the returned image ID to configure
//! the daemon on the same endpoint.
//!
//! API skeleton: methods currently panic; their docs describe intended behavior.

mod archive;
mod environment;
mod error;
mod image;

pub use environment::DockerTestEnvironment;
pub use error::{PreparationError, PreparationStage};
pub use image::{ImagePreparation, PreparedImage};
