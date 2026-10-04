//! Explicit host configuration, constructed with compile-time checked builders.

use std::net::SocketAddr;
use std::path::PathBuf;

use reprise_api::TemplateId;
use typed_builder::TypedBuilder;

/// Configuration for one daemon, one state writer, and one Docker endpoint.
///
/// The daemon is single-node. Remote Docker connectivity does not
/// imply access to that host's filesystem or checkpoint artifacts.
#[derive(Clone, Debug, TypedBuilder)]
pub struct DaemonConfig {
    /// HTTP bind address. Requests are unauthenticated, so a non-loopback
    /// address must be rejected.
    pub listen: SocketAddr,
    /// SQLite database on the daemon host.
    #[builder(setter(into))]
    pub state_db: PathBuf,
    /// Storage for verified, committed snapshot artifacts.
    pub snapshots: SnapshotStorageConfig,
    /// Trusted operator configuration, not sandbox-request parameters.
    pub docker: DockerConfig,
    /// The only environment definition; there is no template registry.
    pub default_template: FixedTemplate,
}

/// Connection to a Docker Engine.
///
/// The endpoint is interpreted by Bollard. Connectivity
/// does not imply access to that machine's filesystem or checkpoint artifacts.
#[derive(Clone, Debug, TypedBuilder)]
pub struct DockerConfig {
    /// Explicit endpoint URI interpreted by Bollard.
    ///
    /// For example, `unix:///var/run/docker.sock` or `ssh://user@host`.
    #[builder(setter(into))]
    pub host: String,
    /// Required sandbox runtime, normally `runsc`. Never fall back to runc.
    #[builder(setter(into))]
    pub runtime: String,
}

/// One fixed environment definition; no template registry is implied.
#[derive(Clone, Debug, TypedBuilder)]
pub struct FixedTemplate {
    /// The only template identifier accepted; requests naming any other
    /// template must be refused, not silently mapped to this one.
    #[builder(setter(into))]
    pub id: TemplateId,
    /// Docker image reference. Pin a digest for reproducible continuity tests.
    #[builder(setter(into))]
    pub image: String,
}

/// Storage owned by the daemon, distinct from runtime checkpoint staging.
#[derive(Clone, Debug, TypedBuilder)]
pub struct SnapshotStorageConfig {
    /// Directory on the daemon host for immutable, committed snapshots.
    ///
    /// This is not a remote Docker checkpoint directory, and checkpoints
    /// written on a remote Docker host are not imported into it.
    #[builder(setter(into))]
    pub committed_dir: PathBuf,
}
