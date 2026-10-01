//! Operator-facing command-line shape. No runtime is started by parsing it.
//!
//! Conversion into validated host configuration is intentionally deferred.

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;

/// Start a single-node Reprise daemon (implementation pending).
#[derive(Clone, Debug, Parser)]
#[command(name = "reprised", version, about)]
pub struct Cli {
    /// HTTP bind address; initial deployments use loopback and SSH forwarding.
    #[arg(long, default_value = "127.0.0.1:8080")]
    pub listen: SocketAddr,

    /// SQLite database on the daemon host.
    #[arg(long, default_value = "./reprise.db")]
    pub state_db: PathBuf,

    /// Committed snapshot directory on the daemon host, not the Docker host.
    #[arg(long, default_value = "./snapshots")]
    pub snapshot_dir: PathBuf,

    /// Local Docker socket or the local end of an external SSH socket tunnel.
    #[arg(long, default_value = "/var/run/docker.sock")]
    pub docker_socket: PathBuf,

    /// Required sandbox runtime; never silently replaced with runc.
    #[arg(long, default_value = "runsc")]
    pub runtime: String,

    /// Only template identifier accepted by the initial fixed-image backend.
    #[arg(long, default_value = "default")]
    pub template_id: String,

    /// Fixed sandbox image; specify a digest for reproducible tests.
    #[arg(long)]
    pub image: String,
}
