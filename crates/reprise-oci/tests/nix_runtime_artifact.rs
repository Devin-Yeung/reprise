//! Contract tests for the Nix output Reprise accepts as a runtime artifact.

use std::fs;
use std::path::Path;

use reprise_oci::nix::RuntimeArtifact;
use reprise_oci::{Layer, Rootfs};
use serde_json::json;

#[test]
fn runtime_artifact_mounts_its_closure_and_command_profile() {
    let store = tempfile::tempdir().unwrap();
    let profile = store.path().join("reprise-command-profile");
    fs::create_dir_all(profile.join("bin")).unwrap();
    let dependency = store.path().join("memory-state");
    fs::create_dir(&dependency).unwrap();

    let artifact_directory = tempfile::tempdir().unwrap();
    fs::write(
        artifact_directory.path().join("manifest.json"),
        format!(
            r#"{{"format_version":1,"command_profile":"{}"}}"#,
            profile.display()
        ),
    )
    .unwrap();
    fs::write(
        artifact_directory.path().join("store-paths"),
        format!("{}\n{}\n", profile.display(), dependency.display()),
    )
    .unwrap();

    let artifact = RuntimeArtifact::load(artifact_directory.path()).unwrap();
    let root = tempfile::tempdir().unwrap();
    let rootfs = Rootfs::new(root.path())
        .unwrap()
        .with_layers([Layer::from(&artifact)])
        .unwrap();

    let mounts = rootfs.mounts();
    assert_eq!(mounts.len(), 3);
    assert_eq!(mounts[0].destination(), &profile);
    assert_eq!(mounts[1].destination(), &dependency);
    assert_eq!(mounts[2].destination(), Path::new("/bin"));
    assert_eq!(mounts[2].source().as_deref(), Some(profile.join("bin").as_path()));
    assert_eq!(
        json!(mounts[2].options()),
        json!(["bind", "ro"]),
        "the command namespace must not let a sandbox modify its Nix profile"
    );

    rootfs.prepare().unwrap();
    assert!(root.path().join("bin").is_dir());
    assert!(root
        .path()
        .join(profile.strip_prefix("/").unwrap())
        .is_dir());
}
