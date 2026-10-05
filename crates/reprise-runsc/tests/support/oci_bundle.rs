//! OCI bundle construction shared by runsc integration tests.

use std::fs;
use std::path::{Path, PathBuf};

use oci_spec::runtime::{Process, Spec};
use reprise_oci::{Layer, Rootfs, nix::RuntimeArtifact};

/// An OCI bundle whose temporary rootfs remains alive while runsc uses it.
pub struct PreparedBundle {
    pub bundle_dir: tempfile::TempDir,
    /// The public command name that runsc resolves through the workload PATH.
    pub memory_state_program: PathBuf,
}

impl PreparedBundle {
    /// Builds a bundle that resolves `memory-state` through the test runtime's
    /// public command profile.
    ///
    /// `layers` express each workload's runtime filesystem contract, stacked
    /// over the closure's store objects. The version command needs none, while
    /// the socket server asks for a writable `/run` tmpfs.
    pub fn memory_state(arguments: &[&str], layers: impl IntoIterator<Item = Layer>) -> Self {
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

        let memory_state_program = PathBuf::from("memory-state");
        save_config(
            &rootfs,
            bundle_dir.path(),
            &memory_state_program,
            &path_environment,
            arguments,
        );

        Self {
            bundle_dir,
            memory_state_program,
        }
    }

    /// Path to the bundle directory.
    pub fn path(&self) -> &Path {
        self.bundle_dir.path()
    }
}

impl AsRef<Path> for PreparedBundle {
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

fn save_config(
    rootfs: &Rootfs,
    bundle_path: &Path,
    program: &Path,
    path_environment: &str,
    arguments: &[&str],
) {
    let mut process = Process::default();
    process
        .set_args(Some(
            std::iter::once(program.to_str().expect("UTF-8").to_owned())
                .chain(arguments.iter().map(|&arg| arg.to_owned()))
                .collect(),
        ))
        .set_env(Some(vec![path_environment.to_owned()]))
        .set_cwd(PathBuf::from("/"));

    let mut spec = Spec::default();
    spec.set_root(Some(rootfs.root().clone()))
        .set_mounts(Some(rootfs.mounts()))
        .set_process(Some(process));
    spec.save(bundle_path.join("config.json"))
        .expect("failed to save bundle config.json");
}
