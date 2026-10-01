//! # Reprise
//!
//! Pause an environment. Resume its state.
//!
//! This is the umbrella crate. It carries no logic of its own; it re-exports the
//! rest of the family so downstream users can depend on `reprise` alone.

pub use reprise_api::*;
