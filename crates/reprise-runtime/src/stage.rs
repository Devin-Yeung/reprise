/// A timed step of an operation, recorded as a `tracing` span named
/// [`name`](Self::name), so a subscriber can tell where the time went.
///
/// [`ColdBoot`](Self::ColdBoot) and [`Restore`](Self::Restore) contain
/// [`Bundle`](Self::Bundle) and [`Network`](Self::Network), then either
/// [`RunscCreate`](Self::RunscCreate) and [`RunscStart`](Self::RunscStart) or
/// [`RunscRestore`](Self::RunscRestore). [`Checkpoint`](Self::Checkpoint)
/// contains [`RunscCheckpoint`](Self::RunscCheckpoint) and
/// [`Commit`](Self::Commit).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stage {
    /// All of [`Runtime::cold_boot`](crate::Runtime::cold_boot).
    ColdBoot,
    /// All of [`Runtime::restore`](crate::Runtime::restore).
    Restore,
    /// All of [`Instance::checkpoint`](crate::Instance::checkpoint).
    Checkpoint,
    /// Writing the OCI bundle.
    Bundle,
    /// Creating the network namespace and veth pair.
    Network,
    RunscCreate,
    RunscStart,
    RunscRestore,
    RunscCheckpoint,
    /// Recording the snapshot's metadata and making its directory visible.
    Commit,
}

impl Stage {
    /// The span name, such as `runsc.restore`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::ColdBoot => "cold_boot",
            Self::Restore => "restore",
            Self::Checkpoint => "checkpoint",
            Self::Bundle => "bundle",
            Self::Network => "network",
            Self::RunscCreate => "runsc.create",
            Self::RunscStart => "runsc.start",
            Self::RunscRestore => "runsc.restore",
            Self::RunscCheckpoint => "runsc.checkpoint",
            Self::Commit => "commit",
        }
    }
}
