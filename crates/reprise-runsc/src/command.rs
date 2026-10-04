//! Pure command construction, separate from subprocess execution and stdio.
//!
//! Container IDs here are already validated by the public interface; tests use
//! literal IDs so argument-contract failures remain independent of validation.

#![expect(
    dead_code,
    reason = "tests-first scaffold; public command methods are not wired yet"
)]

use crate::{
    CheckpointOptions, CreateOptions, DeleteOptions, Invocation, RestoreOptions, RunscConfig,
};

#[derive(Debug)]
pub(crate) enum Command<'a> {
    Version,
    Create {
        id: &'a str,
        options: &'a CreateOptions,
    },
    Start {
        id: &'a str,
    },
    Run {
        id: &'a str,
        options: &'a CreateOptions,
        detached: bool,
    },
    Checkpoint {
        id: &'a str,
        options: &'a CheckpointOptions,
    },
    Restore {
        id: &'a str,
        options: &'a RestoreOptions,
    },
    State {
        id: &'a str,
    },
    List,
    Wait {
        id: &'a str,
    },
    Delete {
        id: &'a str,
        options: DeleteOptions,
    },
}

/// Returns native argv without touching the filesystem or starting runsc.
pub(crate) fn invocation(config: &RunscConfig, command: Command<'_>) -> Invocation {
    // TODO: implement against the reviewed command snapshots, then wire each
    // public method through this builder before adding subprocess execution.
    todo!("runsc argument construction")
}

#[cfg(test)]
mod tests;
