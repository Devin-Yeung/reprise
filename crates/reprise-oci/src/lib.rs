//! Describes the filesystem of an OCI bundle as a [`Rootfs`]: a read-only root
//! directory with a stack of mount [`Layer`]s (tmpfs, `/proc`, `/sys`, binds).
//!
//! [`nix::NixClosure`] reads a local closure artifact and converts into a layer
//! of read-only binds, so Nix store objects are one layer among others. The
//! caller adds process and runtime configuration to its spec. No files are
//! copied and no mounts are performed here; [`Rootfs::prepare`] only creates
//! mount targets on the host.
//!
//! The artifact producer supplies a complete closure for the target architecture.
//! The caller keeps its objects available and unchanged for containers and saved
//! snapshots, including retaining Nix GC roots. This crate does not invoke Nix.

mod error;
pub mod filesystem;
pub mod nix;

pub use error::Error;
pub use filesystem::{Bind, Layer, Rootfs, RootfsError};
