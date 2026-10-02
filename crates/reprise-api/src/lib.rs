//! # Reprise API
//!
//! The public interface of Reprise, and the single source of truth for what a
//! caller can do and what it observes. This crate contains domain types and the
//! [`SandboxService`] trait only — no runtime, storage or transport logic.
//!
//! A sandbox keeps one identity across suspend and resume; the container behind
//! it may change. Callers drive commands and file operations through that
//! identity, and a request is recorded as accepted before it is admitted to
//! run, so giving up the wait does not cancel work already committed.
//!
//! ## Reading this interface
//!
//! A caller creates a [`Sandbox`], works with it through commands ([`Execute`])
//! and file operations ([`ReadFile`], [`WriteFile`]), and requests lifecycle
//! transitions ([`Suspend`], [`Resume`], [`Destroy`]) that it observes as
//! [`Operation`]s.
//!
//! Three rules shape every type here:
//!
//! - A [`SandboxId`] is stable and outlives every physical instance behind it.
//!   [`Sandbox::generation`] changes when that instance changes.
//! - Accepting a request is separate from admitting it. An accepted
//!   [`Execution`] exists before it can run.
//! - A command's non-zero exit code is a result, not an [`Error`].

pub mod capabilities;
pub mod error;
pub mod execution;
pub mod files;
pub mod id;
pub mod operation;
pub mod sandbox;
pub mod service;

pub use capabilities::{Capabilities, RuntimeInfo};
pub use error::Error;
pub use execution::{
    Channel, EventPage, Execute, ExecuteBuilder, Execution, ExecutionEvent, ExecutionState,
};
pub use files::{Digest, FileContent, FileVersion, ReadFile, WriteFile};
pub use id::{ExecutionId, IdempotencyKey, OperationId, SandboxId, SnapshotId, TemplateId};
pub use operation::{
    Destroy, DestroyBuilder, Operation, OperationKind, OperationResult, OperationState, Resume,
    ResumeBuilder, Suspend,
};
pub use sandbox::{
    CreateSandbox, CreateSandboxBuilder, Limits, Sandbox, SandboxInfo, SandboxState,
};
pub use service::SandboxService;
