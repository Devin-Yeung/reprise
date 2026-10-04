//! Opaque identifiers.
//!
//! Each id is a distinct newtype so one kind cannot be passed where another is
//! expected. The service mints sandbox, snapshot, operation and execution ids;
//! callers only echo them back. The constructors exist for implementations of
//! [`SandboxService`](crate::SandboxService) and for decoding ids received over
//! a transport.

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }
    };
}

pub(crate) use string_id;

string_id! {
    /// Stable identity of a sandbox.
    ///
    /// Survives every suspend, resume and physical instance change. Never reused,
    /// including after the sandbox is destroyed.
    SandboxId
}

string_id! {
    /// Identity of an environment definition. A new version is a new `TemplateId`.
    TemplateId
}

string_id! {
    /// Identity of a committed, immutable snapshot.
    SnapshotId
}

string_id! {
    /// Identity of a lifecycle operation. Stable from acceptance.
    OperationId
}

string_id! {
    /// Identity of a command execution. Stable from acceptance.
    ExecutionId
}

string_id! {
    /// A caller-supplied key that makes a mutating request idempotent.
    ///
    /// The same key with the same payload returns the original resource; the same
    /// key with a different payload is an [`Error::IdempotencyConflict`].
    ///
    /// ```
    /// use reprise_api::{IdempotencyKey, Resume};
    ///
    /// let request = Resume::builder().idempotency_key("resume-1").build();
    ///
    /// assert_eq!(request.idempotency_key, Some(IdempotencyKey::new("resume-1")));
    /// ```
    ///
    /// [`Error::IdempotencyConflict`]: crate::Error::IdempotencyConflict
    IdempotencyKey
}
