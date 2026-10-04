//! What the runtime has actually proven it can do.
//!
//! These values come from a startup probe, not from the presence of a binary.
//! A missing capability is refused with
//! [`Error::UnsupportedCapability`](crate::Error::UnsupportedCapability); it is
//! never silently degraded. In particular, a runtime that cannot checkpoint or
//! restore must not cold-boot and report success.

use std::collections::{BTreeMap, BTreeSet};

/// One ability a runtime may prove during the startup probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Capability {
    /// A suspended sandbox can be restored into the same container.
    SameContainerRestore,
    /// Process memory and kernel state can be checkpointed and restored.
    ProcessCheckpoint,
    /// Workspace writes are captured by a snapshot.
    WorkspaceSnapshot,
    /// A running execution can be canceled.
    ExecCancel,
    /// Execution output can be streamed with resumable cursors.
    OutputStream,
}

/// The probed abilities of the configured runtime.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    /// Capabilities the probe proved. Anything absent is unsupported.
    pub supported: BTreeSet<Capability>,
    /// The runtime the probe ran against, when known.
    pub runtime: Option<RuntimeInfo>,
}

impl Capabilities {
    pub fn supports(&self, capability: Capability) -> bool {
        self.supported.contains(&capability)
    }
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
