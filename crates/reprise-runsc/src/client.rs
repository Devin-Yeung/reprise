use std::process::ExitStatus;

use crate::{
    CheckpointOptions, ContainerExit, ContainerId, ContainerState, CreateOptions, DeleteOptions,
    Error, RestoreOptions, RunscConfig, Version,
};

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
        todo!()
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
        todo!()
    }

    /// Lists container IDs in this client's runsc state root.
    pub fn list(&self) -> Result<Vec<ContainerId>, Error> {
        todo!()
    }

    /// Waits for the container's initial process and decodes runsc's JSON result.
    /// The returned code is the workload result, distinct from the wait CLI's status.
    pub fn wait(&self, id: &ContainerId) -> Result<ContainerExit, Error> {
        todo!()
    }

    /// Deletes runtime state. Force deletion also terminates a running container.
    /// It does not remove the caller's bundle, store objects, or checkpoints.
    pub fn delete(&self, id: &ContainerId, options: DeleteOptions) -> Result<(), Error> {
        todo!()
    }
}
