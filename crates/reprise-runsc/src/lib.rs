//! Typed, blocking access to the `runsc` command-line runtime.
//!
//! The caller supplies an OCI bundle directory containing `config.json` and
//! keeps its rootfs, mounts, and checkpoint files available. This crate owns
//! CLI argument construction, subprocess execution, and response decoding;
//! it does not build bundles, configure host networking, probe readiness, or
//! delete containers on drop. Container IDs identify runsc containers, not
//! application-level sandbox identities.
//!
//! # Interface draft
//!
//! Command methods and container-ID validation are unimplemented and panic.
//! Types can be reviewed and examples compiled on macOS; executing runsc needs
//! Linux and the privileges required by the chosen runsc configuration.
//! Argument-contract tests and expected snapshots are wired to a private builder
//! stub and deliberately fail until it is implemented.
//! TODO: implement argv generation against those tests, then subprocess execution
//! and lifecycle validation against a pinned runsc on Linux.
//! TODO: add command timeouts/cancellation before using this interface for a
//! supervised service; blocking commands currently have no deadline contract.
//!
//! Only the launch, inspection, checkpoint, and cleanup operations needed for
//! the first execution experiment are modeled. Exec, signal delivery, console
//! sockets, FD passing, and filesystem-only checkpoints are deferred.

#![expect(
    unused_variables,
    reason = "interface draft; command bodies are deferred"
)]

mod client;
mod command;
mod config;
mod error;
mod options;
mod types;

pub use client::Runsc;
pub use config::{
    FileAccess, GlobalOptions, Network, Overlay, OverlayBacking, Platform, RunscConfig,
};
pub use error::{Error, InvalidContainerId, Invocation, Operation};
pub use options::{
    CheckpointOptions, Compression, ContainerIo, CreateOptions, DeleteOptions, OutputTarget,
    RestoreOptions,
};
pub use types::{ContainerExit, ContainerId, ContainerState, ContainerStatus, Version};
