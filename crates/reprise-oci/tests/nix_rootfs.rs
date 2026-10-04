use std::fs;

use reprise_oci::nix::NixClosure;
use serde_json::json;

#[test]
fn prepares_rootfs_with_readonly_store_mounts() {
    let artifact = tempfile::tempdir().unwrap();
    let manifest = artifact.path().join("store-paths");
    fs::write(
        &manifest,
        concat!("/nix/store/AAA\n", "/nix/store/BBB\n", "/nix/store/CCC\n",),
    )
    .unwrap();
    let base = tempfile::tempdir().unwrap();
    let closure = NixClosure::load(&manifest).unwrap();

    let rootfs = closure.to_rootfs(base.path()).unwrap();
    let spec = json!({
        "root": rootfs.oci_root(),
        "mounts": rootfs.oci_mounts(),
    });

    assert!(base.path().join("nix/store").is_dir());
    assert_eq!(
        rootfs.oci_root().path(),
        &fs::canonicalize(base.path()).unwrap()
    );
    // Only the temporary base path varies between runs; store paths remain literal.
    insta::assert_json_snapshot!(spec, { ".root.path" => "[rootfs]" });
}
