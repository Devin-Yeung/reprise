---
status: draft
date: 2026-10-04
---

# Snapshot runtime: measure and shorten restore

Phase 1 is a performance study: how fast a gVisor sandbox comes back from a
snapshot, where that time goes, and which changes shorten it. The deliverable is
a reproducible benchmark, a baseline broken down by stage, and a report on a few
optimizations with before/after numbers. Anything not on that path is deferred.

Reprise drives `runsc` directly ([ADR 0001](../adr/0001-drive-runsc-directly.md)).
Terms such as instance, cold boot, restore and time to first response are defined
in [`CONTEXT.md`](../../CONTEXT.md).

## Concepts

Study notes for these words, with the primary sources and the place each one
shows up in the runtime, are in [`docs/learn`](../learn/README.md).

- **gVisor, runsc.** gVisor is an application kernel written in Go; `runsc` is its
  OCI runtime CLI, in the role `runc` plays for ordinary containers. A sandbox is
  a few host processes: the *Sentry* implements Linux syscalls for the
  application, the *Gofer* gives it access to host files.
- **Platform.** How the Sentry intercepts the application's syscalls. `systrap`
  (default) uses seccomp traps and needs no hardware virtualization. `kvm` uses
  hardware virtualization; on WSL2 that is nested virtualization.
- **OCI bundle.** A directory holding `config.json` (process, mounts, namespaces)
  and a root filesystem. Docker and containerd normally build it from an image;
  here Reprise builds it.
- **Checkpoint.** `runsc checkpoint` writes the Sentry's state (`checkpoint.img`)
  and the application's memory (separate pages files) into a directory, then
  stops the sandbox.
- **Restore.** `runsc restore` starts a new sandbox from a bundle and loads that
  state instead of running the process from the beginning. It needs a matching
  bundle, the same runsc binary, and a compatible CPU.
- **Background restore.** With `--background`, `runsc restore` returns before all
  memory is loaded and loads the rest while the application runs. It needs an
  uncompressed snapshot. This is gVisor's closest match to the microVM demand
  paging in [research 06](../research/substrate-snapshot-optimizations/06-demand-page-the-working-set.md).
- **Page cache.** The host kernel keeps recently read file data in RAM, so a
  snapshot read from the cache restores far faster than one read from disk.
  Writing `3` to `/proc/sys/vm/drop_caches` empties the cache; `--direct`
  (O_DIRECT) bypasses it.
- **Zero pages.** Memory pages containing only zeros.
  `--exclude-committed-zero-pages` leaves them out of the snapshot.
- **Overlay.** gVisor puts a writable layer over the read-only rootfs
  (`--overlay2`). With `root:memory` that layer lives in sandbox memory, so file
  writes are saved in the snapshot like everything else.
- **Network namespace, veth.** A network namespace is a separate network stack; a
  veth pair is a virtual cable between two namespaces. One end stays on the host,
  so the benchmark can send HTTP requests to the workload.

## Scope

**In:** cold boot, checkpoint and local restore of one instance at a time; time
to first response and per-stage timings; the optimization candidates below.

**Out for phase 1**, each with the reason it was cut:

- *Sandbox service layer* (stable `SandboxId`, persisted records, per-sandbox
  serialization, failure states). It is not on the restore path. Add it when a
  caller needs identity across processes.
- *Exec API.* Workloads answer HTTP, so probing needs no `runsc exec`, whose own
  cost would also blur measurements.
- *Durability.* No fsync; a host crash may lose or corrupt snapshots.
- *Image preparation* (pulling and unpacking OCI images, layer sharing from
  [research 01](../research/substrate-snapshot-optimizations/01-share-unpacked-image-layers.md)).
  Nix exports closure artifacts; `reprise-oci` generates read-only store mounts
  and a shared base root reference before measurement ([design](nix-rootfs.md)).
- *Remote snapshots* and everything that moves them between machines
  (research 02, 08, 09), plus the local copying and staging steps (03, 04) that
  this layout does not have.
- *runsc tests in CI.* Building workload rootfs in CI is fixture work; runsc
  tests run on the workstation until that changes.

## Shape

- `crates/reprise-runtime`: the library. Its public items and rustdoc are the
  interface; this doc does not restate them. Internally, building the bundle,
  constructing runsc arguments, network setup and snapshot layout are separate
  modules, so arguments and layouts are unit-testable without running runsc.
- `crates/reprise-bench`: a benchmark binary with its own measurement loop.
  criterion does not fit: each sample drops caches, boots a sandbox and waits
  seconds, and needs a per-stage breakdown.
- `crates/reprise-oci`: generates root and store-mount configuration from Nix closure artifacts before
  the benchmark loop. The runtime owns per-instance bundle configuration.
- `nix/` and `flake.nix`: build workloads and export their closure artifacts.

The API is blocking. The benchmark runs one operation at a time; parallelism
within an operation, such as preparing the bundle while prefetching snapshot
files, uses scoped threads inside the library.

## On disk

```text
<state_dir>/snapshots/<id>/
  config.json     the bundle spec the lineage was cold-booted with
  manifest.json   runsc version, platform, checkpoint options, network addresses
  checkpoint.img  and the pages files: whatever runsc wrote
```

A checkpoint is written to `<id>.tmp/` and renamed to `<id>/` once the manifest is
in place, so a directory without `.tmp` is always complete. The file list is read
from the directory, not hardcoded.

## Rootfs and network

**Rootfs.** Nix exports a workload runtime closure as a `store-paths` artifact.
`reprise-oci` reads the closure paths and exposes filesystem configuration
with a shared base root and read-only mounts at their original `/nix/store`
paths ([design](nix-rootfs.md)). No store files are copied. The runtime adds the
process and runtime mounts, and retains the same filesystem configuration for
restore. Sources remain available for the lifetime of snapshots.

The fragment defaults to a read-only root. To keep workload file writes in the
snapshot, the runtime must explicitly make the container root writable with
`--overlay2=root:memory`, while store mounts remain read-only. TODO: validate this
combination in the Linux checkpoint/restore correctness gate.

**Network.** Each instance gets a network namespace and a veth pair with fixed
addresses: host `10.200.0.1`, sandbox `10.200.0.2`. A restore therefore sees the
same addresses as the checkpoint. Fixed addresses are why only one instance runs
at a time. The netns is created per instance in the baseline; creating it ahead of
time is an optimization candidate.

## Measurement

- **Time to first response** runs from calling `cold_boot` or `restore` to the
  first successful `GET /state`, polled from the host.
- **Stages** are `tracing` spans with fixed names, collected by the benchmark:
  `cold_boot` and `restore` contain `bundle`, `network`, then `runsc.create` and
  `runsc.start`, or `runsc.restore`. `checkpoint` contains `runsc.checkpoint` and
  `commit`. The wait after `restore` returns is reported as `ready`.
- **Page cache.** Every configuration runs both cold (caches dropped before each
  sample) and warm. Cold is the headline.
- **Repetitions.** At least 20 samples per configuration, reported as p50 and p95.
- **Platform.** Systrap unless the experiment is the platform.
- **Output.** One JSON file per configuration, holding the samples and the
  environment: commit, runsc version, kernel, platform, options, cache state. The
  report lives in `docs/report/`. The JSON files it cites are committed under
  `docs/report/data/`; other runs go to an ignored directory.
- The workstation is a WSL2 VM, so numbers are compared with each other, not
  quoted as absolutes.

## Workloads

- **W1** `memory-state serve`, a few MB. Measures fixed overhead and is the
  correctness gate. Needs a listen-address flag; it binds `127.0.0.1` today.
- **W2** `memory-state` with an N MiB ballast (N = 64, 256, 1024) filled with zeros
  or random bytes. Shows how restore scales with memory and where zero-page
  exclusion matters. Filling random bytes makes cold boot pay real startup work,
  so W2 also carries the restore-versus-cold-boot comparison.
- **W3** (later): Python importing heavy libraries, a realistic slow cold boot.

## Optimization candidates

Measure the baseline (all options default) first. Then take candidates in order
of the stage that dominates the baseline.

| Candidate | Change | Expected to affect |
| --- | --- | --- |
| Background restore | `RestoreOptions::background` | TTFR for W2 |
| Skip zero pages | `CheckpointOptions::exclude_zero_pages` | snapshot size, restore and checkpoint time |
| Direct I/O | `direct_io` on both sides | cold vs warm cache gap |
| Compression | `CheckpointOptions::compress` | size against speed; rules out background |
| Network ahead of time | create the netns before `restore` is called | fixed overhead (W1) |
| Overlapped preparation | build the bundle while prefetching snapshot files ([research 05](../research/substrate-snapshot-optimizations/05-overlap-restore-preparation.md)) | fixed overhead (W1) |
| Platform | `Platform::Kvm` | everything; may not work nested on WSL2 |

An optimization counts only if the correctness gate passes with it enabled.

## Correctness gate

Cold boot W1, mutate its state, write a file, checkpoint, restore. Then require
the same boot nonce, value and revision, the file still present, and the next
mutation to produce the next revision. This runs on the workstation through the
opt-in `integration-tests` feature and fails, rather than skips, when root, runsc
or the rootfs is missing.

## Repository changes

- Add `reprise-runtime`, `reprise-bench`, `reprise-oci` and workload closure artifacts.
- Remove `reprise`, `reprise-api`, `reprise-daemon` and `reprise-test-support`,
  the `docker-integration` CI job and `publish-test-image.yml`. Retire
  `test-image.md` and `test-support-crate.md` in the same change, and rewrite
  `README.md`.
- CI runs fmt, clippy and unit tests on Ubuntu and macOS. Windows is dropped
  because runsc is Linux-only.

## Workstation setup

Development happens over Remote-SSH on the workstation, so the runtime builds and
runs where runsc is. Setup, once:

- rustup and Nix.
- Passwordless sudo, plus `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="sudo -E"`
  in that machine's `.env`. Cargo builds as the user and only test and benchmark
  binaries run as root, so `target/` stays user-owned.
- Add `.env` to the repository's `.gitignore`. On the Mac it is ignored only
  globally.
- In `.wslconfig`: raise `memory=` (the default is half the host's RAM) and set
  `autoMemoryReclaim=disabled`, so WSL does not drop page cache mid-measurement.

## Milestones

1. `reprise-runtime`: cold boot, checkpoint and restore W1, with the
   correctness gate passing.
2. `reprise-bench` and the baseline for W1 and W2.
3. Optimizations, one at a time, each with a section in the report.

## Open questions

Each of these is answered by running something, not by deciding:

- Does `--overlay2=root:memory` keep file writes across checkpoint and restore?
  (milestone 1)
- Does restore need anything beyond identical veth addresses to bring the network
  back? (milestone 1)
- Is `Platform::Kvm` usable under WSL2's nested virtualization?
