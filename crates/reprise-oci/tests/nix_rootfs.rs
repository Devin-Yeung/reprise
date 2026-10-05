use std::fs;

use reprise_oci::nix::NixClosure;
use reprise_oci::{Layer, Rootfs};
use serde_json::json;

fn closure_of(manifest_lines: &str) -> NixClosure {
    let artifact = tempfile::tempdir().unwrap();
    let manifest = artifact.path().join("store-paths");
    fs::write(&manifest, manifest_lines).unwrap();
    NixClosure::load(&manifest).unwrap()
}

#[test]
fn closure_layer_binds_store_objects_read_only() {
    let closure = closure_of(concat!(
        "/nix/store/AAA\n",
        "/nix/store/BBB\n",
        "/nix/store/CCC\n"
    ));
    let base = tempfile::tempdir().unwrap();

    let rootfs = Rootfs::new(base.path())
        .unwrap()
        .with_layers([Layer::from(&closure)])
        .unwrap();
    let spec = json!({
        "root": rootfs.root(),
        "mounts": rootfs.mounts(),
    });

    // Only the temporary base path varies between runs; store paths remain literal.
    insta::assert_json_snapshot!(spec, { ".root.path" => "[rootfs]" });
}

#[test]
fn prepare_creates_a_mount_point_for_every_store_object() {
    let store = tempfile::tempdir().unwrap();
    let objects = ["AAA", "BBB"].map(|name| store.path().join(name));
    for object in &objects {
        fs::create_dir(object).unwrap();
    }
    let closure = closure_of(
        &objects
            .iter()
            .map(|path| format!("{}\n", path.display()))
            .collect::<String>(),
    );
    let base = tempfile::tempdir().unwrap();
    let rootfs = Rootfs::new(base.path())
        .unwrap()
        .with_layers([Layer::from(&closure)])
        .unwrap();

    rootfs.prepare().unwrap();

    for object in &objects {
        let target = base.path().join(object.strip_prefix("/").unwrap());
        assert!(target.is_dir(), "{target:?} must exist");
    }
}
