#![cfg(all(target_os = "linux", feature = "integration-tests"))]

use std::fs;
use std::path::{Path, PathBuf};

use oci_spec::runtime::{Mount, Process, Spec};
use reprise_oci::nix::NixClosure;
use reprise_runsc::{
    ContainerId, ContainerIo, CreateOptions, DeleteOptions, OutputTarget, Runsc, RunscConfig,
};

#[test]
fn smoke_run_wait_and_delete() {
    let runsc_bin = std::env::var_os("REPRISE_RUNSC_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("runsc"));
    let manifest_path = std::env::var_os("REPRISE_TEST_CLOSURE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("result/store-paths"));

    let closure = NixClosure::load(&manifest_path)
        .unwrap_or_else(|err| panic!("failed to load Nix closure from {manifest_path:?}: {err}"));

    let base_dir = tempfile::tempdir().expect("failed to create base tempdir");
    let rootfs = closure
        .to_rootfs(base_dir.path())
        .expect("failed to prepare rootfs");

    // Locate memory-state in closure store paths
    let test_tools_path = closure
        .store_paths()
        .iter()
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.contains("reprise-test-tools"))
        })
        .unwrap_or_else(|| panic!("reprise-test-tools not found in closure store paths"));
    let binary_path = test_tools_path.join("bin/memory-state");

    // Construct OCI bundle
    let bundle_dir = tempfile::tempdir().expect("failed to create bundle tempdir");
    let mut mounts = rootfs.oci_mounts();

    let mut proc_mount = Mount::default();
    proc_mount
        .set_destination(PathBuf::from("/proc"))
        .set_typ(Some("proc".into()))
        .set_source(Some(PathBuf::from("proc")));
    mounts.push(proc_mount);

    let mut dev_mount = Mount::default();
    dev_mount
        .set_destination(PathBuf::from("/dev"))
        .set_typ(Some("tmpfs".into()))
        .set_source(Some(PathBuf::from("tmpfs")));
    mounts.push(dev_mount);

    let mut process = Process::default();
    process
        .set_args(Some(vec![
            binary_path.to_str().unwrap().to_string(),
            "version".to_string(),
        ]))
        .set_cwd(PathBuf::from("/"));

    let mut spec = Spec::default();
    spec.set_root(Some(rootfs.oci_root()))
        .set_mounts(Some(mounts))
        .set_process(Some(process));

    let config_path = bundle_dir.path().join("config.json");
    spec.save(&config_path)
        .expect("failed to save bundle config.json");

    // Configure runsc client and stdio capture
    let state_root = tempfile::tempdir().expect("failed to create state root tempdir");
    let runsc = Runsc::new(RunscConfig {
        executable: runsc_bin,
        state_root: state_root.path().to_path_buf(),
        options: Default::default(),
    });

    let stdout_file = tempfile::NamedTempFile::new().expect("failed to create stdout tempfile");
    let io = ContainerIo::builder()
        .stdout(OutputTarget::File(stdout_file.path().to_path_buf()))
        .build();

    let create_options = CreateOptions::builder()
        .bundle(bundle_dir.path())
        .io(io)
        .build();

    let id = ContainerId::generate();

    // Run container detached, wait for exit, verify output, and delete
    runsc
        .run_detached(&id, &create_options)
        .expect("run_detached failed");

    let exit = runsc.wait(&id).expect("wait failed");
    assert_eq!(exit.code, 0, "workload should exit with code 0");

    let stdout = fs::read_to_string(stdout_file.path()).expect("failed to read stdout file");
    assert!(
        stdout.contains("0.1.0"),
        "expected stdout to contain version 0.1.0, got {stdout:?}"
    );

    runsc
        .delete(&id, DeleteOptions { force: true })
        .expect("delete failed");
}
