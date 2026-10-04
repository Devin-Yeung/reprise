//! Hosting shape, without a daemon implementation.

use std::net::SocketAddr;

use reprise_api::SandboxService;

use crate::config::DaemonConfig;

/// Lifecycle of the process-local daemon host.
///
/// An implementation owns the HTTP listener, lifecycle tasks, SQLite
/// writer, and Docker connection. This is a hosting interface, not a runtime
/// backend registry or a replacement for [`SandboxService`].
pub trait Daemon: Sized + Send {
    /// The single public sandbox interface hosted by this daemon.
    type Service: SandboxService;

    /// Open durable state, connect to Docker, probe capabilities, reconcile
    /// existing records, then bind the listener.
    ///
    /// Unproven checkpoint or workspace capabilities must be reported as
    /// unavailable, never substituted with a cold boot.
    fn start(config: DaemonConfig) -> impl Future<Output = Result<Self, StartupError>> + Send;

    /// Access the same interface used by transport adapters.
    fn service(&self) -> &Self::Service;

    /// Actual bound address, including the selected port when configured as 0.
    fn address(&self) -> SocketAddr;

    /// Stop accepting requests and leave accepted work durably recoverable.
    /// Shutdown does not mean destroying sandboxes or deleting snapshots.
    fn shutdown(self) -> impl Future<Output = Result<(), ShutdownError>> + Send;
}

/// Hosting failures, separate from individual sandbox-request errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartupError {
    InvalidConfig { field: String, reason: String },
    StateUnavailable { reason: String },
    SnapshotStorageUnavailable { reason: String },
    RuntimeUnavailable { reason: String },
    ReconciliationFailed { reason: String },
    ListenerUnavailable { reason: String },
}

/// Failure to cleanly stop the daemon host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShutdownError {
    /// Durable work or background-task termination could not be confirmed.
    Incomplete { reason: String },
}
