use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::InvalidContainerId;

/// One container's identifier in a runsc state root.
///
/// This interface accepts ASCII letters/digits followed by ASCII letters,
/// digits, `_`, `.`, `+`, or `-`. This intentionally excludes leading option
/// markers and filesystem separators, even if some runsc versions allow them.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContainerId(String);

impl ContainerId {
    /// Validates an identifier without probing runsc state.
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidContainerId> {
        todo!()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Strings reported by `runsc --version`; neither is assumed to be SemVer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub runtime: String,
    pub oci_spec: String,
}

/// Decoded `runsc state` response; PID absence is distinct from a live PID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainerState {
    pub oci_version: String,
    pub id: ContainerId,
    pub status: ContainerStatus,
    pub pid: Option<u32>,
    pub bundle: PathBuf,
    pub annotations: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContainerStatus {
    Creating,
    Created,
    Running,
    Stopped,
    /// Preserves runtime extensions rather than inventing lifecycle semantics.
    Other(String),
}

/// Decoded `runsc wait` response, independent of the wait command's exit status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainerExit {
    pub id: ContainerId,
    /// runsc encodes a signaled exit as `128 + signal`; this is not a raw Linux
    /// wait status and cannot unambiguously distinguish signals from exit codes.
    pub code: u32,
}
