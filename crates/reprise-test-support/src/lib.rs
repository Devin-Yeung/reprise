//! Prepare immutable registry images on a real, already connected Docker Engine.
//! Only a missing local reference triggers a pull; shared images are never removed.

mod image;

pub use image::{ImagePreparation, PrepareError, PreparedImage, ensure_image};
