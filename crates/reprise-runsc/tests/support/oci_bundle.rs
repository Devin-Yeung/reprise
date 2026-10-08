//! Test-image preparation shared by runsc integration tests.

use std::fs;
use std::path::{Path, PathBuf};

use reprise_oci::{Layer, Rootfs, nix::RuntimeArtifact};

/// A prepared test image whose temporary rootfs remains alive while runsc uses it.
///
/// The image deliberately contains no process configuration. Callers choose the
/// OCI process, including its program, arguments, and any other spec fields.
pub struct PreparedImage {
    bundle_dir: tempfile::TempDir,
    rootfs: Rootfs,
    path_environment: String,
}

impl PreparedImage {
    /// Prepares the test runtime image with `layers` stacked above its closure.
    ///
    /// The caller writes `config.json` using [`Self::rootfs`] and
    /// [`Self::path_environment`]. This keeps the image independent of a
    /// particular test workload.
    pub fn create(layers: impl IntoIterator<Item = Layer>) -> Self {
        let runtime = test_runtime();
        let path_environment = runtime.path_environment();
        let bundle_dir = tempfile::tempdir().expect("failed to create bundle tempdir");
        let rootfs_dir = bundle_dir.path().join("rootfs");
        fs::create_dir_all(&rootfs_dir).expect("failed to create rootfs dir");

        let rootfs = Rootfs::new(&rootfs_dir)
            .and_then(|rootfs| {
                rootfs.with_layers(std::iter::once(Layer::from(&runtime)).chain(layers))
            })
            .expect("failed to compose OCI rootfs");
        rootfs.prepare().expect("failed to prepare OCI rootfs");

        Self {
            bundle_dir,
            rootfs,
            path_environment,
        }
    }

    /// Directory where the caller writes the OCI `config.json`.
    pub fn path(&self) -> &Path {
        self.bundle_dir.path()
    }

    /// Root filesystem and mounts to place in the caller's OCI spec.
    pub fn rootfs(&self) -> &Rootfs {
        &self.rootfs
    }

    /// `PATH` environment entry for commands exported by the test runtime.
    pub fn path_environment(&self) -> &str {
        &self.path_environment
    }
}

impl AsRef<Path> for PreparedImage {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

fn test_runtime() -> RuntimeArtifact {
    let artifact_directory = std::env::var_os("REPRISE_TEST_RUNTIME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("result"));
    RuntimeArtifact::load(&artifact_directory).unwrap_or_else(|err| {
        panic!("failed to load Nix runtime artifact from {artifact_directory:?}: {err}")
    })
}
