//! Contract test for the runtime artifact built by the real Nix expression.

#![cfg(not(target_os = "windows"))]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use reprise_oci::nix::RuntimeArtifact;
use reprise_oci::{Layer, Rootfs};
use serde_json::{Value, json};

#[test]
fn nix_runtime_artifact_prepares_the_complete_rootfs() {
    let artifact = RuntimeArtifact::load(build_test_runtime()).unwrap();
    let root = tempfile::tempdir().unwrap();
    let rootfs = Rootfs::new(root.path())
        .unwrap()
        .with_layers([Layer::from(&artifact)])
        .unwrap();

    rootfs.prepare().unwrap();

    let mut environment = json!({
        "path_environment": artifact.path_environment(),
        "root": rootfs.root(),
        "mounts": rootfs.mounts(),
        "prepared_paths": paths_below(root.path()),
    });
    // Store object names vary by target platform and Nix version. Numbering
    // them preserves each path's relationships while keeping one snapshot for
    // every supported host platform.
    name_store_objects(&mut environment, &mut HashMap::new());
    insta::assert_json_snapshot!(environment, {
        ".root.path" => "[rootfs]",
    });
}

fn build_test_runtime() -> PathBuf {
    let output = Command::new("nix")
        .args([
            "build",
            "--no-link",
            "--print-out-paths",
            ".#reprise-test-runtime",
        ])
        .current_dir(workspace_root())
        .output()
        .expect("Nix must be installed to build the runtime artifact");
    assert!(
        output.status.success(),
        "nix build .#reprise-test-runtime failed:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );

    let paths = String::from_utf8(output.stdout)
        .expect("nix build must print a UTF-8 output path")
        .lines()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    assert_eq!(paths.len(), 1, "Nix must produce one runtime artifact");
    paths.into_iter().next().unwrap()
}

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("reprise-oci must remain below the workspace root")
}

fn paths_below(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_paths(root, root, &mut paths);
    paths
}

fn name_store_objects(value: &mut Value, store_objects: &mut HashMap<String, usize>) {
    match value {
        Value::String(path) => {
            if let Some(store_path) = path.strip_prefix("/nix/store/") {
                let index = store_path.find('/').unwrap_or(store_path.len());
                let (store_object, suffix) = store_path.split_at(index);
                let number = match store_objects.get(store_object) {
                    Some(number) => *number,
                    None => {
                        let number = store_objects.len() + 1;
                        store_objects.insert(store_object.to_owned(), number);
                        number
                    }
                };
                *path = format!("/nix/store/[store object {number}]{suffix}");
            }
        }
        Value::Array(values) => values
            .iter_mut()
            .for_each(|value| name_store_objects(value, store_objects)),
        Value::Object(values) => values
            .values_mut()
            .for_each(|value| name_store_objects(value, store_objects)),
        _ => {}
    }
}

fn collect_paths(root: &Path, directory: &Path, paths: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(directory)
        .unwrap()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        paths.push(Path::new("/").join(path.strip_prefix(root).unwrap()));
        if entry.file_type().unwrap().is_dir() {
            collect_paths(root, &path, paths);
        }
    }
}
