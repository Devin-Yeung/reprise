//! What the runtime has actually proven it can do.
//!
//! These values come from a startup probe, not from the presence of a binary.
//! A false capability is refused with
//! [`Error::UnsupportedCapability`](crate::Error::UnsupportedCapability); it is
//! never silently degraded. In particular, a runtime that cannot checkpoint or
//! restore must not cold-boot and report success.

use std::collections::BTreeMap;

/// The probed abilities of the configured runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capabilities {
    /// A suspended sandbox can be restored into the same container.
    pub same_container_restore: bool,
    /// Process memory and kernel state can be checkpointed and restored.
    pub process_checkpoint: bool,
    /// Workspace writes are captured by a snapshot.
    pub workspace_snapshot: bool,
    /// A running execution can be canceled.
    pub exec_cancel: bool,
    /// Execution output can be streamed with resumable cursors.
    pub output_stream: bool,
    /// The runtime the probe ran against, when known.
    pub runtime: Option<RuntimeInfo>,
}

/// The runtime the probe ran against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeInfo {
    /// The container engine, for example `docker`.
    pub engine: String,
    /// The sandbox runtime, for example `runsc`.
    pub sandbox: String,
    /// Versions observed during the probe, for example `runsc`.
    pub versions: BTreeMap<String, String>,
}
