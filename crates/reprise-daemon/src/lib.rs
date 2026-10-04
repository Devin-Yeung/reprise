//! Configuration and hosting interface for the Reprise daemon.
//!
//! This crate is a shape-only skeleton. It does not connect to Docker, serve
//! HTTP, persist state, or create snapshots. The binary exposes CLI help and
//! rejects startup until those implementations exist.
//!
//! Callers continue to use [`reprise_api::SandboxService`]. Hosting configuration
//! is separate from sandbox requests; callers cannot choose arbitrary Docker
//! runtimes, mounts, or privileged settings.

pub mod cli;
pub mod config;
pub mod daemon;

pub use config::{DaemonConfig, DockerConfig, FixedTemplate, SnapshotStorageConfig};
pub use daemon::{Daemon, ShutdownError, StartupError};
