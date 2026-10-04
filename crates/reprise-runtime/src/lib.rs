//! # Reprise runtime
//!
//! Starts gVisor sandboxes from scratch or from a snapshot, and checkpoints them
//! into snapshots, by building OCI bundles and invoking the `runsc` CLI directly.
//! It exists to measure and shorten restore, so it exposes the runsc options
//! under study and reports where an operation spends its time as [`Stage`]s.
//!
//! The API is blocking. A [`Runtime`] runs one [`Instance`] at a time, and one
//! `Runtime` exists per host, because every instance uses the same network
//! addresses. Starting an instance needs Linux and root.
//!
//! A benchmark sample, timing restore until the workload first answers:
//!
//! ```no_run
//! use std::time::Instant;
//!
//! use reprise_runtime::{CheckpointOptions, RestoreOptions, Runtime, RuntimeConfig, Workload};
//! # fn drop_page_cache() -> std::io::Result<()> { Ok(()) }
//! # fn wait_until_ready(_: std::net::IpAddr) -> std::io::Result<()> { Ok(()) }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut runtime = Runtime::new(RuntimeConfig {
//!     runsc: "/usr/local/bin/runsc".into(),
//!     state_dir: "/var/lib/reprise".into(),
//!     platform: Default::default(),
//! })?;
//! let workload = Workload {
//!     rootfs: reprise_oci::nix::NixClosure::load("/var/lib/reprise-closure/store-paths")?,
//!     args: ["/nix/store/<hash>-reprise-test-tools/bin/memory-state", "serve", "--listen", "0.0.0.0:8765"].map(Into::into).into(),
//!     env: Vec::new(),
//! };
//! let snapshot = runtime.cold_boot(&workload)?.checkpoint(&CheckpointOptions::default())?;
//!
//! let mut samples = Vec::new();
//! for _ in 0..20 {
//!     drop_page_cache()?;
//!     let start = Instant::now();
//!     let instance = runtime.restore(&snapshot, &RestoreOptions::default())?;
//!     wait_until_ready(instance.address())?;
//!     samples.push(start.elapsed());
//!     instance.destroy()?;
//! }
//! # Ok(())
//! # }
//! ```

#![expect(
    unused_variables,
    dead_code,
    reason = "interface skeleton without bodies"
)]

mod error;
mod instance;
mod runtime;
mod snapshot;
mod stage;

pub use error::{Error, Incompatibility};
pub use instance::{CheckpointOptions, Instance};
pub use runtime::{Platform, RestoreOptions, Runtime, RuntimeConfig, Workload};
pub use snapshot::Snapshot;
pub use stage::Stage;
