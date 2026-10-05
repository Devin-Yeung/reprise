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

    /// Generates a random valid container identifier prefixed with `reprise-`.
    pub fn generate() -> Self {
        todo!()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ContainerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for ContainerId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_container_ids() {
        for id in [
            "a",
            "0",
            "Z",
            "container1",
            "12345",
            "a-b_c.1+2",
            "reprise-benchmark.1",
        ] {
            assert_eq!(ContainerId::new(id).unwrap().as_str(), id);
        }
    }

    #[test]
    fn rejected_container_ids() {
        for id in [
            "",
            "-foo",
            "--detach",
            "_foo",
            ".foo",
            "+foo",
            "foo/bar",
            "/root",
            "..",
            "foo bar",
            " ",
            "容器",
            "café",
            "foo@bar",
            "foo:bar",
            "foo=bar",
        ] {
            assert_eq!(
                ContainerId::new(id),
                Err(InvalidContainerId {
                    value: id.to_owned()
                })
            );
        }
    }

    #[test]
    fn generated_id_is_valid_and_unique() {
        let id1 = ContainerId::generate();
        let id2 = ContainerId::generate();
        assert_ne!(id1, id2);
        assert!(id1.as_str().starts_with("reprise-"));
        assert_eq!(ContainerId::new(id1.as_str()), Ok(id1));
        assert_eq!(ContainerId::new(id2.as_str()), Ok(id2));
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
