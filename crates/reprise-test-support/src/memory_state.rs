//! The `memory-state` workload built into the fixture image.
//!
//! Every subcommand (`start`, `state`, `mutate VALUE`) prints one
//! [`MemoryState`] as a JSON line. `start` spawns a detached HTTP server whose
//! `boot_nonce` is fixed for its lifetime, so an unchanged nonce proves the
//! same process kept running.

use serde::Deserialize;

use crate::is_sha256_hex;

/// Absolute path of the workload binary inside the fixture image.
pub const MEMORY_STATE: &str = "/bin/memory-state";

/// The server's in-memory state as reported by one subcommand.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct MemoryState {
    /// 64 lowercase hex characters chosen when the server started.
    pub boot_nonce: String,
    pub value: Option<String>,
    pub revision: u64,
}

impl MemoryState {
    /// Whether `boot_nonce` has the documented 64-character lowercase hex form.
    pub fn has_valid_boot_nonce(&self) -> bool {
        is_sha256_hex(&self.boot_nonce)
    }
}
