//! Explicit host configuration, constructed with compile-time checked builders.

use std::net::SocketAddr;
use std::path::PathBuf;

use bon::Builder;

/// Configuration for one daemon, one state writer, and one Docker endpoint.
///
/// The initial deployment is single-node. Remote Docker connectivity does not
/// imply access to that host's filesystem or checkpoint artifacts.
#[derive(Clone, Debug, Builder)]
pub struct DaemonConfig {
    /// HTTP bind address. Exposing a non-loopback listener requires an
    /// authentication policy; the first implementation should reject it.
    pub listen: SocketAddr,
    /// SQLite database on the daemon host.
    pub state_db: PathBuf,
    /// Storage for verified, committed snapshot artifacts.
    pub snapshots: SnapshotStorageConfig,
    /// Trusted operator configuration, not sandbox-request parameters.
    pub docker: DockerConfig,
    /// The only supported environment definition in the first version.
    pub default_template: FixedTemplate,
}

/// Connection to a Docker Engine using a Unix socket.
///
/// For remote development, this may be the local end of an externally managed
/// SSH tunnel. Docker CLI contexts and SSH sessions are not managed here.
#[derive(Clone, Debug, Builder)]
pub struct DockerConfig {
    /// Unix socket on the daemon host.
    pub socket: PathBuf,
    /// Required sandbox runtime, normally `runsc`. Never fall back to runc.
    pub runtime: String,
}

/// One fixed environment definition; no template registry is implied.
#[derive(Clone, Debug, Builder)]
pub struct FixedTemplate {
    /// External template identifier. Conversion to the domain TemplateId and
    /// validation are deferred; unknown identifiers must not be silently ignored.
    pub id: String,
    /// Docker image reference. Pin a digest for reproducible continuity tests.
    pub image: String,
}

/// Storage owned by the daemon, distinct from runtime checkpoint staging.
#[derive(Clone, Debug, Builder)]
pub struct SnapshotStorageConfig {
    /// Directory on the daemon host for immutable, committed snapshots.
    ///
    /// This is not a remote Docker checkpoint directory. Importing remote
    /// artifacts requires a separate transfer mechanism that is not yet defined.
    pub committed_dir: PathBuf,
}
