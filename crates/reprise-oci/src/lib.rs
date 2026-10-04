//! Describes local Nix closures as OCI filesystem configuration.
//!
//! [`nix::NixClosure`] reads a closure artifact. Its `to_rootfs` method creates or
//! reuses the caller's base directory and returns a [`nix::NixBasedRootFS`] that
//! exposes a read-only [`Root`] and [`Mount`]s for the closure. The caller adds
//! process and runtime configuration to its spec.
//! No files are copied and no mounts are performed here.
//!
//! The artifact producer supplies a complete closure for the target architecture.
//! The caller keeps its objects available and unchanged for containers and saved
//! snapshots, including retaining Nix GC roots. This crate does not invoke Nix.
//!
//! [`Root`]: oci_spec::runtime::Root
//! [`Mount`]: oci_spec::runtime::Mount

mod error;
pub mod nix;

pub use error::Error;
