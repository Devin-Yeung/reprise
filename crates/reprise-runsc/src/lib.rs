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
//! Pure argv construction and container-ID validation are implemented.
//! Public command methods in `client.rs` are still unimplemented and panic.
//! Executing runsc needs Linux and the privileges required by the chosen
//! configuration.
//! TODO: implement the methods in `client.rs`: build each `Command`, execute its
//! `Invocation` without a shell, route launch stdio, and handle
//! operation-specific status and output. Then validate lifecycle behavior
//! against a pinned runsc on Linux.
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
