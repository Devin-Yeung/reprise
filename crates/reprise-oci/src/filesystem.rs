//! The filesystem portion of an OCI bundle: one read-only root directory and a
//! stack of [`Layer`]s mounted over it.
//!
//! A layer here is a group of mounts, not an OCI image layer. Layers apply
//! bottom to top, and [`Rootfs`] checks that the stack is unambiguous.
//!
//! ```
//! use reprise_oci::{Layer, Rootfs};
//!
//! let rootfs = Rootfs::new("/srv/rootfs")?.with_layers([Layer::proc(), Layer::tmpfs("/run")])?;
//! assert_eq!(rootfs.mounts().len(), 2);
//! # Ok::<(), reprise_oci::RootfsError>(())
//! ```

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use oci_spec::runtime::{Mount, Root};

/// A read-only root directory plus layers in the order a runtime applies them.
///
/// Invariants hold at every step, so a `Rootfs` is always ready to insert into
/// a bundle spec: the root path is absolute, every mount destination stays
/// inside the root, no two mounts share a destination, and no mount hides an
/// earlier one. Nested destinations are fine when the parent comes first.
///
/// Combining two root directories is not expressible.
#[derive(Clone, Debug)]
pub struct Rootfs {
    root: Root,
    layers: Vec<Layer>,
}

/// A rootfs cannot represent the supplied path or layers unambiguously.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RootfsError {
    /// A rootfs must name an absolute host directory.
    #[error("rootfs path must be absolute, got {path:?}")]
    RelativeRoot { path: PathBuf },
    /// A mount destination must be an absolute path contained by the rootfs.
    #[error("mount destination must be an absolute path below the rootfs, got {destination:?}")]
    InvalidDestination { destination: PathBuf },
    /// More than one mount targets the same container path.
    #[error("multiple mounts target {destination:?}")]
    DuplicateDestination { destination: PathBuf },
    /// A mount would cover an earlier mount below it.
    #[error("mount at {destination:?} would hide an earlier mount below it")]
    HidesEarlierMount { destination: PathBuf },
}

impl Rootfs {
    /// A read-only root at an absolute host directory, with no layers.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, RootfsError> {
        let path = path.into();
        if !path.is_absolute() {
            return Err(RootfsError::RelativeRoot { path });
        }
        let mut root = Root::default();
        root.set_path(path).set_readonly(Some(true));
        Ok(Self {
            root,
            layers: vec![],
        })
    }

    /// Stacks layers above those already present, preserving their order.
    pub fn with_layers(
        mut self,
        layers: impl IntoIterator<Item = Layer>,
    ) -> Result<Self, RootfsError> {
        let mut destinations = HashSet::new();
        // Strict ancestors of every destination so far; a later mount on one
        // of them would cover what is below.
        let mut ancestors = HashSet::new();
        let existing = self.layers.iter().flat_map(Layer::mounts);
        let incoming: Vec<Layer> = layers.into_iter().collect();
        for mount in existing.chain(incoming.iter().flat_map(Layer::mounts)) {
            let destination = mount.destination();
            validate_destination(destination)?;
            if destinations.contains(destination) {
                return Err(RootfsError::DuplicateDestination {
                    destination: destination.clone(),
                });
            }
            if ancestors.contains(destination) {
                return Err(RootfsError::HidesEarlierMount {
                    destination: destination.clone(),
                });
            }
            ancestors.extend(destination.ancestors().skip(1).map(Path::to_path_buf));
            destinations.insert(destination.clone());
        }
        self.layers.extend(incoming);
        Ok(self)
    }

    /// Prepares the host for the runtime to mount every layer.
    ///
    /// Run this before the runtime makes the root read-only. It creates mount
    /// targets below the root directory and fails if a bind source is missing.
    pub fn prepare(&self) -> io::Result<()> {
        for layer in &self.layers {
            layer.prepare(self.root.path())?;
        }
        Ok(())
    }

    /// The root passed to an OCI runtime.
    pub fn root(&self) -> &Root {
        &self.root
    }

    /// Layers from bottom to top.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// Every layer's mounts in the order an OCI runtime must apply them.
    pub fn mounts(&self) -> Vec<Mount> {
        self.layers.iter().flat_map(Layer::mounts).collect()
    }
}

/// A group of mounts that a [`Rootfs`] stacks as one unit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Layer {
    /// A writable in-memory filesystem, e.g. at `/run` for Unix sockets.
    Tmpfs { destination: PathBuf },
    /// The process filesystem at `/proc`.
    Proc,
    /// A read-only view of kernel objects at `/sys`.
    Sys,
    /// Host paths exposed at container paths.
    Binds(Vec<Bind>),
}

impl Layer {
    /// A tmpfs at `destination`.
    pub fn tmpfs(destination: impl Into<PathBuf>) -> Self {
        Self::Tmpfs {
            destination: destination.into(),
        }
    }

    /// `/proc`.
    pub fn proc() -> Self {
        Self::Proc
    }

    /// Read-only `/sys`.
    pub fn sys() -> Self {
        Self::Sys
    }

    /// One layer of bind mounts, applied in the given order.
    pub fn binds(binds: impl IntoIterator<Item = Bind>) -> Self {
        Self::Binds(binds.into_iter().collect())
    }

    /// The OCI mounts this layer expands to, in application order.
    pub fn mounts(&self) -> Vec<Mount> {
        match self {
            Self::Tmpfs { destination } => vec![mount(destination, "tmpfs", "tmpfs", &[])],
            Self::Proc => vec![mount(Path::new("/proc"), "proc", "proc", &[])],
            Self::Sys => vec![mount(
                Path::new("/sys"),
                "sysfs",
                "sysfs",
                &["nosuid", "noexec", "nodev", "ro"],
            )],
            Self::Binds(binds) => binds.iter().map(Bind::mount).collect(),
        }
    }

    fn prepare(&self, root: &Path) -> io::Result<()> {
        match self {
            Self::Binds(binds) => binds.iter().try_for_each(|bind| bind.prepare(root)),
            Self::Tmpfs { destination } => create_dir_target(root, destination),
            Self::Proc => create_dir_target(root, Path::new("/proc")),
            Self::Sys => create_dir_target(root, Path::new("/sys")),
        }
    }
}

/// One host path exposed at a container path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bind {
    source: PathBuf,
    destination: PathBuf,
    writable: bool,
}

impl Bind {
    /// Exposes `source` at `destination` without allowing writes.
    pub fn read_only(source: impl Into<PathBuf>, destination: impl Into<PathBuf>) -> Self {
        Self {
            source: source.into(),
            destination: destination.into(),
            writable: false,
        }
    }

    /// Exposes `source` at `destination`, allowing writes to the host path.
    pub fn read_write(source: impl Into<PathBuf>, destination: impl Into<PathBuf>) -> Self {
        Self {
            writable: true,
            ..Self::read_only(source, destination)
        }
    }

    fn mount(&self) -> Mount {
        let access = if self.writable { "rw" } else { "ro" };
        mount(&self.destination, "bind", &self.source, &["bind", access])
    }

    /// A directory source needs a directory target and a file source needs a
    /// file, or the runtime cannot bind one onto the other.
    fn prepare(&self, root: &Path) -> io::Result<()> {
        let metadata = fs::metadata(&self.source).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("bind source {:?}: {error}", self.source),
            )
        })?;
        if metadata.is_dir() {
            return create_dir_target(root, &self.destination);
        }
        let target = host_path(root, &self.destination);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(target)
            .map(drop)
    }
}

fn mount(
    destination: impl Into<PathBuf>,
    typ: &str,
    source: impl Into<PathBuf>,
    options: &[&str],
) -> Mount {
    let mut mount = Mount::default();
    mount
        .set_destination(destination.into())
        .set_typ(Some(typ.into()))
        .set_source(Some(source.into()));
    if !options.is_empty() {
        mount.set_options(Some(options.iter().map(|&o| o.to_owned()).collect()));
    }
    mount
}

fn create_dir_target(root: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(host_path(root, destination))
}

/// Maps a validated container path below the root directory. Removing the one
/// leading slash makes it relative, so the result cannot leave the root.
fn host_path(root: &Path, destination: &Path) -> PathBuf {
    root.join(
        destination
            .strip_prefix("/")
            .expect("validated mount destination"),
    )
}

fn validate_destination(destination: &Path) -> Result<(), RootfsError> {
    let is_contained = destination.is_absolute()
        && destination.components().all(|component| {
            !matches!(
                component,
                Component::CurDir | Component::ParentDir | Component::Prefix(_)
            )
        });
    if is_contained {
        Ok(())
    } else {
        Err(RootfsError::InvalidDestination {
            destination: destination.to_path_buf(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use oci_spec::runtime::Mount;

    use super::{Bind, Layer, Rootfs, RootfsError};

    fn destinations(mounts: &[Mount]) -> Vec<PathBuf> {
        mounts
            .iter()
            .map(|mount| mount.destination().clone())
            .collect()
    }

    fn rootfs() -> Rootfs {
        Rootfs::new("/workload-root").unwrap()
    }

    #[test]
    fn root_is_read_only() {
        let rootfs = rootfs();

        assert_eq!(rootfs.root().path(), Path::new("/workload-root"));
        assert_eq!(rootfs.root().readonly(), Some(true));
    }

    #[test]
    fn rejects_relative_rootfs_path() {
        let error = Rootfs::new("relative/root").expect_err("a rootfs must be an absolute path");

        assert_eq!(
            error,
            RootfsError::RelativeRoot {
                path: PathBuf::from("relative/root"),
            }
        );
    }

    #[test]
    fn expands_layers_into_mounts_in_stacking_order() {
        let rootfs = rootfs()
            .with_layers([Layer::binds([Bind::read_only("/a", "/a")]), Layer::proc()])
            .unwrap()
            .with_layers([Layer::tmpfs("/run"), Layer::sys()])
            .unwrap();

        assert_eq!(
            destinations(&rootfs.mounts()),
            ["/a", "/proc", "/run", "/sys"].map(PathBuf::from)
        );
    }

    #[test]
    fn layers_expand_to_standard_oci_mounts() {
        let mounts = |layer: Layer| {
            serde_json::to_value(layer.mounts()).expect("mounts must serialize to JSON")
        };

        assert_eq!(
            mounts(Layer::proc()),
            serde_json::json!([{"destination": "/proc", "type": "proc", "source": "proc"}])
        );
        assert_eq!(
            mounts(Layer::sys()),
            serde_json::json!([{
                "destination": "/sys", "type": "sysfs", "source": "sysfs",
                "options": ["nosuid", "noexec", "nodev", "ro"],
            }])
        );
        assert_eq!(
            mounts(Layer::tmpfs("/run")),
            serde_json::json!([{"destination": "/run", "type": "tmpfs", "source": "tmpfs"}])
        );
        assert_eq!(
            mounts(Layer::binds([
                Bind::read_only("/host/ro", "/ro"),
                Bind::read_write("/host/rw", "/rw"),
            ])),
            serde_json::json!([
                {"destination": "/ro", "type": "bind", "source": "/host/ro", "options": ["bind", "ro"]},
                {"destination": "/rw", "type": "bind", "source": "/host/rw", "options": ["bind", "rw"]},
            ])
        );
    }

    #[test]
    fn accepts_nested_destinations_parent_first() {
        rootfs()
            .with_layers([Layer::tmpfs("/var"), Layer::tmpfs("/var/lib/workload")])
            .expect("a child mounted after its parent stays visible");
    }

    #[test]
    fn rejects_duplicate_destinations_across_layers_and_calls() {
        let duplicate = RootfsError::DuplicateDestination {
            destination: PathBuf::from("/run"),
        };

        let within_call = rootfs().with_layers([Layer::tmpfs("/run"), Layer::tmpfs("/run")]);
        let across_calls = rootfs()
            .with_layers([Layer::tmpfs("/run")])
            .unwrap()
            .with_layers([Layer::tmpfs("/run")]);

        assert_eq!(within_call.unwrap_err(), duplicate);
        assert_eq!(across_calls.unwrap_err(), duplicate);
    }

    #[test]
    fn rejects_a_mount_that_would_hide_an_earlier_one() {
        let error = rootfs()
            .with_layers([Layer::tmpfs("/var/lib/workload")])
            .unwrap()
            .with_layers([Layer::tmpfs("/var")])
            .expect_err("a parent mounted later covers its earlier child");

        assert_eq!(
            error,
            RootfsError::HidesEarlierMount {
                destination: PathBuf::from("/var"),
            }
        );
    }

    #[test]
    fn rejects_mount_destinations_that_could_escape_the_rootfs() {
        for destination in ["/run/../../host", "relative"] {
            let error = rootfs()
                .with_layers([Layer::tmpfs(destination)])
                .expect_err("a mount target must be absolute and stay below the rootfs");

            assert_eq!(
                error,
                RootfsError::InvalidDestination {
                    destination: PathBuf::from(destination),
                }
            );
        }
    }

    #[test]
    fn prepares_mount_targets_for_every_layer() {
        let dir = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let rootfs = Rootfs::new(dir.path())
            .unwrap()
            .with_layers([
                Layer::proc(),
                Layer::tmpfs("/var/lib/workload"),
                Layer::binds([Bind::read_only(source.path(), "/data")]),
            ])
            .unwrap();

        rootfs
            .prepare()
            .expect("preparation must create missing directories");

        for target in ["proc", "var/lib/workload", "data"] {
            assert!(dir.path().join(target).is_dir(), "{target} must exist");
        }
    }

    #[test]
    fn prepares_a_file_target_for_a_file_bind() {
        let dir = tempfile::tempdir().unwrap();
        let source_dir = tempfile::tempdir().unwrap();
        let source = source_dir.path().join("script.sh");
        std::fs::write(&source, "#!/bin/sh\n").unwrap();
        let rootfs = Rootfs::new(dir.path())
            .unwrap()
            .with_layers([Layer::binds([Bind::read_only(&source, "/bin/script.sh")])])
            .unwrap();

        rootfs.prepare().unwrap();

        assert!(dir.path().join("bin/script.sh").is_file());
    }

    #[test]
    fn prepare_fails_when_a_bind_source_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let rootfs = Rootfs::new(dir.path())
            .unwrap()
            .with_layers([Layer::binds([Bind::read_only("/does/not/exist", "/data")])])
            .unwrap();

        let error = rootfs.prepare().expect_err("a missing source must fail");

        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        assert!(error.to_string().contains("/does/not/exist"), "{error}");
    }
}
