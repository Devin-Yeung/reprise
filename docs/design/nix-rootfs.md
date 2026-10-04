---
status: draft
date: 2026-10-05
---

# Prepare a rootfs from a Nix closure artifact

Phase 1 needs a repeatable filesystem from which runsc can start the test
workload. Nix already builds the programs and determines their runtime closure.
Rust copies that closure into a rootfs on the execution host. This preparation
happens once before the benchmark loop, outside cold-boot and restore timings.

## Terms and ownership

Closure artifact, rootfs and bundle are defined in
[`CONTEXT.md`](../../CONTEXT.md#filesystem-preparation). The runtime owns the
command, mounts and network configuration in the bundle's `config.json`.

`reprise-oci` owns filesystem preparation. Its first implementation is the
`nix` module. It is separate from `reprise-runtime` because the artifact is
prepared before an instance exists and reused across its cold boots and restores.
The model distinguishes an artifact's dependency metadata, the loaded Nix
closure of store objects, and the prepared rootfs. These are different stages:
loading metadata does not imply the objects exist, and a rootfs is returned only
after copying succeeds. Additional providers can be introduced when a workload
actually requires them.

The public interface and its preconditions are documented in
[`NixClosure`](../../crates/reprise-oci/src/nix.rs) and
[`Rootfs`](../../crates/reprise-oci/src/rootfs.rs). The caller passes the
`store-paths` file itself; loading reads its paths in order, and the artifact
producer supplies complete absolute paths. Materialization copies those objects into a new destination and returns a
prepared rootfs with an absolute host path. The runtime continues to accept
`Workload { rootfs, args, env }`.
Command selection is independent of closure selection: one package may contain
several programs, and a workload may include several packages.

## Artifact and filesystem layout

The flake exports `reprise-test-closure` using Nixpkgs
[`closureInfo`](https://github.com/NixOS/nixpkgs/blob/master/pkgs/build-support/closure-info.nix).
Its `store-paths` file is the only metadata the copier consumes: one absolute
store-object path per line. The roots are package outputs, so the list describes
runtime dependencies rather than the closure of build derivations. Multiple
roots produce the union of their closures without choosing an entry command.

```text
Nix build output                    Materialized rootfs
...-closure-info/                   <destination>/
  store-paths                         nix/store/
  registration                          <hash>-reprise-test-tools/
  total-nar-size                         <hash>-dependency/...
```

Store objects keep their original `/nix/store/...` paths inside the rootfs.
Preserving symbolic links and embedded loader paths therefore needs no rewriting
or flattening into `/bin`. The command uses its absolute store path inside the
sandbox. The host destination may be anywhere runsc can access.

Copying uses Rust filesystem APIs on Unix hosts. Symbolic links are retained
verbatim, including dangling links; hard links within the closure retain their
identity without linking back to the host store. Directory permissions are
applied after copying children so read-only store directories can be populated.
Unsupported file types fail explicitly. Owners, timestamps and extended
attributes are outside this Nix-content copier's preservation contract; it is
not a general filesystem archive copier.

The artifact and its store objects are already local and remain unchanged while
preparation runs. This module does not build, fetch or register a Nix store.
Nix does not need to be installed inside the resulting filesystem.

The source packages must target the Linux architecture where runsc executes.
A closure built for macOS is useful for checking preparation, not for running
the sandbox. Workload-specific configuration files or writable directories are
supplied explicitly; runtime mounts such as `/proc` and `/dev` remain runtime
configuration. Reprise keeps the base filesystem unchanged and uses
`--overlay2=root:memory` for instance writes, as specified by the
[runtime design](snapshot-runtime.md#rootfs-and-network).

## Development setup

Build and prepare on the Linux execution host:

```sh
nix build .#reprise-test-closure --out-link result
cargo run --locked -p reprise-oci --example nix-rootfs -- result/store-paths /tmp/reprise-rootfs
```

The destination is new and its parent exists. Missing store objects and a
destination inside a source directory fail before creating the destination.
A failed copy leaves a partial destination for the caller to remove. This is
fixture preparation, so the first implementation uses ordinary copying and reports errors without a staging or
retry protocol. OCI image pulling/unpacking and layer caching remain outside
phase 1.

The preparation implementation and example are in
[`reprise-oci`](../../crates/reprise-oci). The design stays `draft` pending
integration with the Linux runtime's correctness gate. TODO: supply its
store-path command and required runtime mounts when implementing cold boot,
then run the checkpoint/restore gate.
