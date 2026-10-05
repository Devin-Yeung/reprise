---
status: draft
date: 2026-10-05
---

# Compose a rootfs from a shared base and local Nix store mounts

Workloads on one execution host share immutable Nix store objects. Preparing an
instance must not duplicate those objects. `reprise-oci` reads a closure artifact
and expresses it as a layer of read-only bind mounts, retaining each object's
original `/nix/store` path. The layer stacks on a `Rootfs` with other layers
(see [composable-filesystems.md](composable-filesystems.md)). It neither copies
files nor performs mounts. See the public interface in
[`NixClosure`](../../crates/reprise-oci/src/nix.rs).

## Ownership and layout

The caller selects a shared base directory and builds a `Rootfs` over it;
`Rootfs::prepare` creates the mount point for each store object. The base may
contain only a directory skeleton. Each instance has its own bundle
configuration and writable state, while the base and store objects are reused. Store objects are mounted individually rather than exposing
the execution host's entire store.

```text
shared base directory         OCI filesystem configuration
  nix/store/                    root.path = rootfs (caller selects shared base), readonly = true
                                mounts:
                                  /nix/store/<object> -> /nix/store/<object>, ro
```

The fragment does not choose a program. The runtime merges it with process,
networking, `/dev` and isolation policy to create the complete bundle;
`/proc`, `/sys` and writable directories are further layers on the same `Rootfs`. A read-only base configuration
and a writable overlay are different policies: the runtime must explicitly
configure root writability if using gVisor's per-instance memory overlay.

## Runtime artifact and command profile

A Nix closure says which store objects are available. It does not say which
commands Reprise promises to downstream callers, nor does it construct a
conventional command directory. A **runtime artifact** is the build output
that supplies both pieces of information:

```text
runtime artifact
  manifest.json     format version and command-profile store path
  store-paths       complete runtime closure, one absolute path per line
```

The producer builds a command profile with `buildEnv`. It selects the packages
whose `bin/` entries are part of the workload's public command surface, keeps
Nix's collision detection enabled, and makes that profile the root of
`closureInfo`. The artifact manifest names the profile explicitly; consumers
must not infer it by searching store-path basenames.

At preparation, the runtime mounts every listed store object at its original
`/nix/store` path, then bind-mounts the profile's `bin/` directory read-only at
`/bin`. A command API that accepts bare names also sets `PATH=/bin`; known
entrypoints may use `/bin/<command>` directly. The profile is only a stable
facade: its symlinks resolve into the same mounted closure and introduce no
second dependency set.

`/bin` is a deliberately small OCI-facing command namespace, rather than a
claim that the rootfs is a complete FHS distribution. If Reprise promises POSIX
shell scripts, the profile must deliberately provide and validate `/bin/sh`;
merely exposing `bash` does not make that promise.

## Preconditions and lifetime

The artifact producer supplies a complete runtime closure targeting the Linux
architecture of the execution host. `store-paths` contains one direct absolute
`/nix/store` object path per line. The manifest's command-profile path must
also occur in that list and its `bin/` directory must exist. Metadata loading
and conversion to layers do not copy store objects; the layers list mounts in
manifest order. The producer supplies valid paths and complete dependencies.
`Rootfs::prepare` is the only step that touches the host, and it fails if a
listed object or the command profile is missing.

The caller keeps the base and dependencies available and unchanged for all
instances and saved snapshots, including retaining Nix GC roots. Reading the artifact does
not pin objects or prevent later deletion. Nix build/fetch/registration, image
pulling, closure completeness checks and architecture discovery are outside this
module. The runtime must retain the filesystem fragment for snapshot restore.

## Development and validation

Build a local closure and prepare the shared base before generating configuration:

```sh
nix build .#reprise-test-runtime --out-link result
cargo run --locked -p reprise-oci --example nix-rootfs -- result/store-paths /tmp/reprise-base
```

The example creates the base directory if missing and prints only root and
mounts as JSON, not a complete runnable bundle. Reuse that base across instances.
Configuration tests run without mounting or invoking runsc. The design stays
`draft` until Linux runtime integration verifies the assembled filesystem.
TODO: verify shared-base concurrent instances and checkpoint/restore with
read-only store mounts and isolated writable state.
