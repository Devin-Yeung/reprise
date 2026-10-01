//! Opaque identifiers.
//!
//! Each id is a distinct newtype so one kind cannot be passed where another is
//! expected. The inner representation is deliberately private: only the
//! implementing crate constructs these.

/// Stable identity of a sandbox.
///
/// Survives every suspend, resume and physical instance change. Never reused,
/// including after the sandbox is destroyed.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SandboxId(String);

/// Identity of an environment definition. A new version is a new `TemplateId`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TemplateId(String);

/// Identity of a committed, immutable snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SnapshotId(String);

/// Identity of a lifecycle operation. Stable from acceptance.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OperationId(String);

/// Identity of a command execution. Stable from acceptance.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExecutionId(String);

/// A caller-supplied key that makes a mutating request idempotent.
///
/// The same key with the same payload returns the original resource; the same
/// key with a different payload is an [`Error::IdempotencyConflict`].
///
/// [`Error::IdempotencyConflict`]: crate::Error::IdempotencyConflict
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IdempotencyKey(String);
