use anyhow::{Context, Result};
use reprise_api::{SandboxId, SandboxService};
use serde::Deserialize;

use super::execution::execute_successfully;

// Built into reprise-test-image; no interpreter or inline source.
const MEMORY_STATE: &str = "/bin/memory-state";

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub(super) struct MemoryState {
    pub boot_nonce: String,
    pub value: Option<String>,
    pub revision: u64,
}

pub(super) struct MemoryWorkload<'a, S> {
    service: &'a S,
    sandbox: &'a SandboxId,
}

impl<'a, S: SandboxService> MemoryWorkload<'a, S> {
    pub fn new(service: &'a S, sandbox: &'a SandboxId) -> Self {
        Self { service, sandbox }
    }

    pub async fn start(&self) -> Result<MemoryState> {
        self.command(&["start"]).await
    }

    pub async fn read(&self) -> Result<MemoryState> {
        self.command(&["state"]).await
    }

    pub async fn mutate(&self, value: &str) -> Result<MemoryState> {
        self.command(&["mutate", value]).await
    }

    async fn command(&self, args: &[&str]) -> Result<MemoryState> {
        let argv = std::iter::once(MEMORY_STATE)
            .chain(args.iter().copied())
            .map(str::to_owned)
            .collect();

        let stdout = execute_successfully(self.service, self.sandbox, argv)
            .await
            .with_context(|| format!("memory-state {}", args[0]))?;

        serde_json::from_slice(&stdout).context("invalid memory-state JSON output")
    }
}
