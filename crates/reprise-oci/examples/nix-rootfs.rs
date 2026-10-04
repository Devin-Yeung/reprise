//! Copies a local Nix closure into a new rootfs directory.
//!
//! Build the closure artifact first with `nix build .#reprise-test-closure`, then:
//! `cargo run --locked -p reprise-oci --example nix-rootfs -- result/store-paths /tmp/reprise-rootfs`
//!
//! The destination must not exist and its parent must exist. A failed copy may
//! leave a partial destination; remove it before retrying. To run the prepared
//! workload, build the closure for the Linux architecture of the execution host.

use std::env;
use std::error::Error;
use std::path::PathBuf;

use reprise_oci::nix::NixClosure;

fn main() -> Result<(), Box<dyn Error>> {
    // OS strings let callers use filesystem paths that are not valid UTF-8.
    let mut arguments = env::args_os().skip(1);

    let manifest: PathBuf = arguments
        .next()
        .expect("expect manifest path")
        .try_into()
        .expect("invalid manifest path");

    let dest: PathBuf = arguments
        .next()
        .expect("expect destination path")
        .try_into()
        .expect("invalid destination path");

    let closure = NixClosure::load(&manifest)?;
    let rootfs = closure.materialize(dest.as_ref())?;
    println!("{}", rootfs.as_path().display());
    Ok(())
}
