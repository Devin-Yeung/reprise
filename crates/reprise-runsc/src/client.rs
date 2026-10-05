use std::collections::BTreeMap;
use std::os::fd::AsFd;
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, ExitStatus, Stdio};

use serde::{Deserialize, Deserializer};

use crate::command::{self, Command};
use crate::{
    CheckpointOptions, ContainerExit, ContainerId, ContainerIo, ContainerState, ContainerStatus,
    CreateOptions, DeleteOptions, Error, Invocation, OutputTarget, RestoreOptions, RunscConfig,
    Version,
};

#[derive(Deserialize)]
struct WaitResponse {
    id: String,
    #[serde(rename = "exitStatus")]
    exit_status: u32,
}

#[derive(Deserialize)]
struct StateResponse {
    #[serde(rename = "ociVersion")]
    oci_version: String,
    id: String,
    status: String,
    #[serde(default, deserialize_with = "deserialize_state_pid")]
    pid: Option<u32>,
    bundle: PathBuf,
    #[serde(default)]
    annotations: BTreeMap<String, String>,
}

/// Decodes runsc's stopped-process marker without making it part of our API.
///
/// A state response uses `-1` when no process remains. The public state model
/// represents that condition as `None`; accepting any other negative number
/// would hide a malformed or changed runtime response.
fn deserialize_state_pid<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw_pid = Option::<i64>::deserialize(deserializer)?;
    match raw_pid {
        None | Some(-1) => Ok(None),
        Some(raw_pid) => u32::try_from(raw_pid).map(Some).map_err(|_| {
            serde::de::Error::custom(format!("expected a non-negative u32 or -1, got {raw_pid}"))
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::StateResponse;

    #[test]
    fn stopped_state_treats_runsc_negative_pid_sentinel_as_absent() {
        // runsc writes -1 after the initial process has exited. It is not a
        // process identifier, so callers must observe the same absence they
        // would for an omitted PID rather than a runtime-specific sentinel.
        let response: StateResponse = serde_json::from_str(
            r#"{
                "ociVersion": "1.1.0-rc.1",
                "id": "reprise-18fb9a3128950be5",
                "status": "stopped",
                "pid": -1,
                "bundle": "/tmp/reprise-bundle"
            }"#,
        )
        .expect("runsc stopped-state response must decode");

        assert_eq!(response.pid, None);
    }
}

/// Configures launch stdio without leaking pipes to background sandbox processes.
fn configure_stdio(
    cmd: &mut ProcessCommand,
    io: &ContainerIo,
    invocation: &Invocation,
) -> Result<(), Error> {
    cmd.stdin(Stdio::null());
    match &io.stdout {
        OutputTarget::Inherit => {
            cmd.stdout(Stdio::inherit());
        }
        OutputTarget::Null => {
            cmd.stdout(Stdio::null());
        }
        OutputTarget::File(path) => {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|source| Error::Io {
                    invocation: Box::new(invocation.clone()),
                    source,
                })?;
            cmd.stdout(file);
        }
        OutputTarget::Fd(fd) => {
            let fd = fd
                .as_fd()
                .try_clone_to_owned()
                .map_err(|source| Error::Io {
                    invocation: Box::new(invocation.clone()),
                    source,
                })?;
            cmd.stdout(Stdio::from(fd));
        }
    }
    match &io.stderr {
        OutputTarget::Inherit => {
            cmd.stderr(Stdio::inherit());
        }
        OutputTarget::Null => {
            cmd.stderr(Stdio::null());
        }
        OutputTarget::File(path) => {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|source| Error::Io {
                    invocation: Box::new(invocation.clone()),
                    source,
                })?;
            cmd.stderr(file);
        }
        OutputTarget::Fd(fd) => {
            let fd = fd
                .as_fd()
                .try_clone_to_owned()
                .map_err(|source| Error::Io {
                    invocation: Box::new(invocation.clone()),
                    source,
                })?;
            cmd.stderr(Stdio::from(fd));
        }
    }
    Ok(())
}

/// A runsc executable and configuration shared by its command invocations.
///
/// Each operation invokes a subprocess without a shell. Calls are blocking;
/// success of launch commands does not establish application readiness.
/// This handle has no ownership of containers and performs no drop cleanup.
#[derive(Debug)]
pub struct Runsc {
    config: RunscConfig,
}

impl Runsc {
    /// Stores configuration without probing the binary or changing the filesystem.
    pub fn new(config: RunscConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &RunscConfig {
        &self.config
    }

    /// Queries the executable's reported runtime and OCI specification versions.
    pub fn version(&self) -> Result<Version, Error> {
        todo!()
    }

    /// Creates a container from a prepared bundle without starting its program.
    /// Its stdio is established here and remains in use after `start` returns.
    pub fn create(&self, id: &ContainerId, options: &CreateOptions) -> Result<(), Error> {
        todo!()
    }

    /// Starts an already-created container and returns when the CLI exits.
    pub fn start(&self, id: &ContainerId) -> Result<(), Error> {
        todo!()
    }

    /// Creates and runs a container in the foreground, waiting for CLI exit.
    ///
    /// Returns every CLI exit status, including nonzero ones. The CLI forwards
    /// workload exits but may also fail itself; status alone cannot distinguish
    /// those cases. Diagnostics go to the configured stdio destinations.
    pub fn run(&self, id: &ContainerId, options: &CreateOptions) -> Result<ExitStatus, Error> {
        todo!()
    }

    /// Creates and starts a container with `run --detach`, without waiting for exit.
    pub fn run_detached(&self, id: &ContainerId, options: &CreateOptions) -> Result<(), Error> {
        let invocation = command::invocation(
            &self.config,
            Command::Run {
                id: id.as_str(),
                options,
                detached: true,
            },
        );
        let mut cmd = ProcessCommand::new(&invocation.executable);
        cmd.args(&invocation.args);
        configure_stdio(&mut cmd, &options.io, &invocation)?;

        let status = cmd.status().map_err(|source| Error::Io {
            invocation: Box::new(invocation.clone()),
            source,
        })?;

        if !status.success() {
            return Err(Error::Command {
                invocation: Box::new(invocation),
                status,
                stdout: None,
                stderr: None,
            });
        }

        Ok(())
    }

    /// Saves execution state. The container stops unless `leave_running` is set.
    /// The caller owns checkpoint directory preparation, compatibility metadata,
    /// and filesystem consistency; this is not a snapshot publication operation.
    pub fn checkpoint(&self, id: &ContainerId, options: &CheckpointOptions) -> Result<(), Error> {
        todo!()
    }

    /// Invokes `restore --detach` using the supplied bundle and execution image.
    ///
    /// Container creation behavior follows the installed runsc version. This
    /// wrapper neither checks snapshot compatibility nor falls back to cold boot.
    /// Background loading may keep reading the image after this call returns.
    pub fn restore(&self, id: &ContainerId, options: &RestoreOptions) -> Result<(), Error> {
        todo!()
    }

    /// Reads OCI container state; it does not report application readiness.
    pub fn state(&self, id: &ContainerId) -> Result<ContainerState, Error> {
        let invocation = command::invocation(&self.config, Command::State { id: id.as_str() });
        let output = ProcessCommand::new(&invocation.executable)
            .args(&invocation.args)
            .output()
            .map_err(|source| Error::Io {
                invocation: Box::new(invocation.clone()),
                source,
            })?;

        if !output.status.success() {
            return Err(Error::Command {
                invocation: Box::new(invocation),
                status: output.status,
                stdout: Some(output.stdout),
                stderr: Some(output.stderr),
            });
        }

        let response: StateResponse =
            serde_json::from_slice(&output.stdout).map_err(|err| Error::InvalidOutput {
                invocation: Box::new(invocation.clone()),
                reason: err.to_string(),
                stdout: output.stdout.clone(),
                stderr: output.stderr.clone(),
            })?;

        let container_id = ContainerId::new(&response.id).map_err(|err| Error::InvalidOutput {
            invocation: Box::new(invocation),
            reason: err.to_string(),
            stdout: output.stdout,
            stderr: output.stderr,
        })?;

        let status = match response.status.as_str() {
            "creating" => ContainerStatus::Creating,
            "created" => ContainerStatus::Created,
            "running" => ContainerStatus::Running,
            "stopped" => ContainerStatus::Stopped,
            _ => ContainerStatus::Other(response.status),
        };

        Ok(ContainerState {
            oci_version: response.oci_version,
            id: container_id,
            status,
            pid: response.pid,
            bundle: response.bundle,
            annotations: response.annotations,
        })
    }

    /// Lists container IDs in this client's runsc state root.
    pub fn list(&self) -> Result<Vec<ContainerId>, Error> {
        todo!()
    }

    /// Waits for the container's initial process and decodes runsc's JSON result.
    /// The returned code is the workload result, distinct from the wait CLI's status.
    pub fn wait(&self, id: &ContainerId) -> Result<ContainerExit, Error> {
        let invocation = command::invocation(&self.config, Command::Wait { id: id.as_str() });
        let mut cmd = ProcessCommand::new(&invocation.executable);
        cmd.args(&invocation.args);

        let output = cmd.output().map_err(|source| Error::Io {
            invocation: Box::new(invocation.clone()),
            source,
        })?;

        if !output.status.success() {
            return Err(Error::Command {
                invocation: Box::new(invocation),
                status: output.status,
                stdout: Some(output.stdout),
                stderr: Some(output.stderr),
            });
        }

        let response: WaitResponse =
            serde_json::from_slice(&output.stdout).map_err(|err| Error::InvalidOutput {
                invocation: Box::new(invocation.clone()),
                reason: err.to_string(),
                stdout: output.stdout.clone(),
                stderr: output.stderr.clone(),
            })?;

        let container_id = ContainerId::new(&response.id).map_err(|err| Error::InvalidOutput {
            invocation: Box::new(invocation),
            reason: err.to_string(),
            stdout: output.stdout,
            stderr: output.stderr,
        })?;

        Ok(ContainerExit {
            id: container_id,
            code: response.exit_status,
        })
    }

    /// Deletes runtime state. Force deletion also terminates a running container.
    /// It does not remove the caller's bundle, store objects, or checkpoints.
    pub fn delete(&self, id: &ContainerId, options: DeleteOptions) -> Result<(), Error> {
        let invocation = command::invocation(
            &self.config,
            Command::Delete {
                id: id.as_str(),
                options,
            },
        );
        let mut cmd = ProcessCommand::new(&invocation.executable);
        cmd.args(&invocation.args);

        let output = cmd.output().map_err(|source| Error::Io {
            invocation: Box::new(invocation.clone()),
            source,
        })?;

        if !output.status.success() {
            return Err(Error::Command {
                invocation: Box::new(invocation),
                status: output.status,
                stdout: Some(output.stdout),
                stderr: Some(output.stderr),
            });
        }

        Ok(())
    }
}
