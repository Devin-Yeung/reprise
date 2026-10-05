//! OCI bundle construction shared by runsc integration tests.

use std::fs;
use std::path::{Path, PathBuf};

use oci_spec::runtime::{Process, Spec};
use reprise_oci::{Layer, Rootfs, nix::NixClosure};

/// An OCI bundle whose temporary rootfs remains alive while runsc uses it.
pub struct PreparedBundle {
    pub bundle_dir: tempfile::TempDir,
    pub memory_state: PathBuf,
}

impl PreparedBundle {
    /// Builds a bundle that invokes `memory-state` from the test Nix closure.
    ///
    /// `layers` express each workload's runtime filesystem contract, stacked
    /// over the closure's store objects. The version command needs none, while
    /// the socket server asks for a writable `/run` tmpfs.
    pub fn memory_state(arguments: &[&str], layers: impl IntoIterator<Item = Layer>) -> Self {
        let closure = test_closure();
        let memory_state = memory_state_binary(&closure);
        let bundle_dir = tempfile::tempdir().expect("failed to create bundle tempdir");
        let rootfs_dir = bundle_dir.path().join("rootfs");
        fs::create_dir_all(&rootfs_dir).expect("failed to create rootfs dir");

        let rootfs = Rootfs::new(&rootfs_dir)
            .and_then(|rootfs| {
                rootfs.with_layers(std::iter::once(Layer::from(&closure)).chain(layers))
            })
            .expect("failed to compose OCI rootfs");
        rootfs.prepare().expect("failed to prepare OCI rootfs");

        save_config(&rootfs, bundle_dir.path(), &memory_state, arguments);

        Self {
            bundle_dir,
            memory_state,
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

fn test_closure() -> NixClosure {
    let manifest_path = std::env::var_os("REPRISE_TEST_CLOSURE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("result/store-paths"));
    NixClosure::load(&manifest_path)
        .unwrap_or_else(|err| panic!("failed to load Nix closure from {manifest_path:?}: {err}"))
}

fn memory_state_binary(closure: &NixClosure) -> PathBuf {
    closure
        .store_paths()
        .iter()
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().contains("reprise-test-tools"))
        })
        .expect("reprise-test-tools not found in closure store paths")
        .join("bin/memory-state")
}

fn save_config(rootfs: &Rootfs, bundle_path: &Path, memory_state: &Path, arguments: &[&str]) {
    let mut process = Process::default();
    process
        .set_args(Some(
            std::iter::once(memory_state.to_str().expect("UTF-8").to_owned())
                .chain(arguments.iter().map(|&arg| arg.to_owned()))
                .collect(),
        ))
        .set_cwd(PathBuf::from("/"));

    let mut spec = Spec::default();
    spec.set_root(Some(rootfs.root().clone()))
        .set_mounts(Some(rootfs.mounts()))
        .set_process(Some(process));
    spec.save(bundle_path.join("config.json"))
        .expect("failed to save bundle config.json");
}
