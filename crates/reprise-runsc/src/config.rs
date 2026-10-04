use std::path::PathBuf;

/// Location of runsc and its runtime state, independent of OCI bundle paths.
/// Paths are passed without filesystem preparation; callers should use absolute
/// paths so changing the host working directory cannot change their meaning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunscConfig {
    pub executable: PathBuf,
    /// runsc's `--root`, not the container rootfs or a bundle directory.
    pub state_root: PathBuf,
    pub options: GlobalOptions,
}

/// Typed global flags. `None` leaves the flag unset, retaining runsc's default.
/// Defaults can change across runsc versions; pin the binary for comparisons.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GlobalOptions {
    pub platform: Option<Platform>,
    pub network: Option<Network>,
    pub overlay: Option<Overlay>,
    /// Applies to every bind mount, independently of root filesystem caching.
    pub file_access_mounts: Option<FileAccess>,
    pub directfs: Option<bool>,
}

/// How the Sentry intercepts application system calls (`--platform`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Platform {
    Systrap,
    /// Requires `/dev/kvm` and usable hardware virtualization.
    Kvm,
}

/// runsc network implementation; host namespaces are prepared by the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Network {
    Sandbox,
    /// Provides only sandbox-local loopback networking.
    None,
    /// Uses the host network stack, reducing network isolation.
    Host,
}

/// Bind-mount cache consistency (`--file-access-mounts`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileAccess {
    /// Revalidates cached entries against possible host changes.
    Shared,
    /// Permits aggressive caching; files must not be modified externally.
    /// Concurrent readers are allowed, but deletion by Nix GC is a modification.
    Exclusive,
}

/// Which mounts receive a writable overlay (`--overlay2`).
/// An overlay does not override OCI read-only policy; the bundle controls the
/// application's permission to write. Store mounts can remain read-only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Overlay {
    None,
    Root(OverlayBacking),
    All(OverlayBacking),
}

/// Storage for an overlay's writable layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OverlayBacking {
    /// Uses application memory and increases sandbox memory consumption.
    Memory,
    /// Uses a backing file within the host mount, which must be writable.
    SelfBacked,
    /// Uses a backing file beneath this host directory.
    Directory(PathBuf),
}
