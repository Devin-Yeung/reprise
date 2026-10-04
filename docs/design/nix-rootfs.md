---
status: draft
date: 2026-10-05
---

# Compose a rootfs from a shared base and local Nix store mounts

Workloads on one execution host share immutable Nix store objects. Preparing an
instance must not duplicate those objects. `reprise-oci` reads a closure artifact
and produces OCI filesystem configuration: a root reference and read-only bind
mounts, retaining each object's original `/nix/store` path. It neither copies
files nor performs mounts. See the public interface in
[`NixClosure`](../../crates/reprise-oci/src/nix.rs).

## Ownership and layout

The caller selects a shared base directory. `to_rootfs` creates it and its
`nix/store` directory if missing; the caller supplies individual mount points
and other directories needed by the runtime. It may contain only a directory skeleton. Each instance
has its own bundle configuration and writable state, while the base and store
objects are reused. Store objects are mounted individually rather than exposing
the execution host's entire store.

```text
shared base directory         OCI filesystem configuration
  nix/store/                    root.path = rootfs (caller selects shared base), readonly = true
                                mounts:
                                  /nix/store/<object> -> /nix/store/<object>, ro
```

The fragment does not choose a program or supply runtime mounts. The runtime
merges it with process, networking, `/proc`, `/dev`, writable directories and
isolation policy to create the complete bundle. A read-only base configuration
and a writable overlay are different policies: the runtime must explicitly
configure root writability if using gVisor's per-instance memory overlay.

## Preconditions and lifetime

The artifact producer supplies a complete runtime closure targeting the Linux
architecture of the execution host. `store-paths` contains one direct absolute
`/nix/store` object path per line. Metadata
loading maps each listed path to a read-only bind mount without probing sources.
The producer supplies valid paths and complete dependencies. `to_rootfs` returns
a `NixBasedRootFS` holding the absolute base path and closure. `oci_root` returns
the read-only root configuration, and `oci_mounts` returns mounts in manifest
order. Neither conversion probes or copies store objects.

The caller keeps the base and dependencies available and unchanged for all
instances and saved snapshots, including retaining Nix GC roots. Reading the artifact does
not pin objects or prevent later deletion. Nix build/fetch/registration, image
pulling, closure completeness checks and architecture discovery are outside this
module. The runtime must retain the filesystem fragment for snapshot restore.

## Development and validation

Build a local closure and prepare the shared base before generating configuration:

```sh
nix build .#reprise-test-closure --out-link result
mkdir -p /tmp/reprise-base/nix/store
cargo run --locked -p reprise-oci --example nix-rootfs -- result/store-paths /tmp/reprise-base
```

The example creates the base directory if missing and prints only root and
mounts as JSON, not a complete runnable bundle. Reuse that base across instances.
Configuration tests run without mounting or invoking runsc. The design stays
`draft` until Linux runtime integration verifies the assembled filesystem.
TODO: verify shared-base concurrent instances and checkpoint/restore with
read-only store mounts and isolated writable state.
