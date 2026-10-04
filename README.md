# **Reprise** —  Pause an environment. Resume its state.

Reprise measures how fast a gVisor sandbox comes back from a snapshot, and
shortens it. It drives `runsc` directly
([ADR 0001](docs/adr/0001-drive-runsc-directly.md)); the current plan is
[Snapshot runtime](docs/design/snapshot-runtime.md). The operating-system
background for that plan is in [docs/learn](docs/learn/README.md).

- `crates/reprise-runtime`: cold boot, checkpoint and restore.
- `crates/reprise-runsc`: standalone typed runsc CLI interface (design scaffold;
  command execution is not implemented).
- `crates/reprise-oci`: generate root and read-only mount configuration from local Nix closures
  ([design](docs/design/nix-rootfs.md)).
- `test-tools/`: the `memory-state` workload, built by `nix/`.

Unit tests run on Linux and macOS:

```sh
cargo test --locked --workspace --all-targets
```

Anything that starts a sandbox needs Linux, root and runsc; see the plan's
workstation setup.
