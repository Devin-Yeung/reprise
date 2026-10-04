# Drive runsc directly, without Docker

Reprise's first goal is to measure and shorten snapshot restore. Behind Docker, the
steps worth timing and changing (rootfs preparation, where checkpoint files go,
restore preparation) run inside dockerd, containerd and the shim, where we can
neither instrument nor change them, and every number includes those hops. So
Reprise builds OCI bundles and invokes `runsc` itself, as root on a Linux host.

Docker was first chosen only so a macOS machine could drive a remote Linux host
through a forwarded socket ([research](../research/direct-runsc-vs-docker.md),
section 7). Developing on the Linux host removes that reason.

## Considered options

- **Docker + runsc.** Besides hiding the steps above: restore only into the same
  container, `docker checkpoint` needs the daemon's experimental mode, and runsc
  flags are global per registered runtime.
- **containerd + the runsc shim.** Same problem: the snapshotter and shim own
  rootfs and checkpoint handling.

## Consequences

- Reprise owns what Docker provided: bundle and rootfs, sandbox networking, and
  cleanup of sandboxes left behind by a crashed process.
- Anything that runs `runsc` (integration tests, benchmarks) needs Linux and root;
  macOS only compiles and runs unit tests.
- No Docker comparison number is kept.
