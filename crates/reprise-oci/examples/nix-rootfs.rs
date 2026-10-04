//! Prints a filesystem configuration fragment for a local Nix closure.
//!
//! Build with `nix build .#reprise-test-closure`, then run:
//! `cargo run --locked -p reprise-oci --example nix-rootfs -- result/store-paths /var/lib/reprise/base-rootfs`
//!
//! Creates the base directory if missing, then prints root and mounts, not a
//! complete runnable `config.json`. Reuse the same directory across instances.

use std::env;
use std::error::Error;
use std::path::PathBuf;

use reprise_oci::nix::NixClosure;

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let manifest: PathBuf = arguments.next().expect("expect manifest path").into();

    let root: PathBuf = arguments.next().expect("expect base rootfs path").into();

    let closure = NixClosure::load(manifest)?;
    let rootfs = closure.to_rootfs(root)?;
    let fragment = serde_json::json!({
        "root": rootfs.oci_root(),
        "mounts": rootfs.oci_mounts(),
    });
    println!("{}", serde_json::to_string_pretty(&fragment)?);
    Ok(())
}
