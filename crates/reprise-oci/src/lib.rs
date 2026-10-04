//! Prepares root filesystems for OCI bundles from local artifacts.
//!
//! [`nix::NixClosure`] loads the runtime dependency list from a Nix closure
//! artifact. Materialization copies that closure into a new [`Rootfs`], retaining
//! its `/nix/store` paths. The runtime separately owns the bundle's command,
//! mounts and networking. Preparation happens before startup measurements.
//!
//! This blocking API supports Unix hosts and uses filesystem APIs without
//! invoking Nix or external copy tools. Sources must remain available and
//! unchanged throughout preparation, and target the sandbox's architecture.

mod copy;
mod error;
pub mod nix;
mod rootfs;

pub use error::Error;
pub use rootfs::Rootfs;
