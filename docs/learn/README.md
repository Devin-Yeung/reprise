# Concepts behind snapshot restore

These notes are the background for the words in [Snapshot runtime](../design/snapshot-runtime.md) and in the [Substrate snapshot series](../research/substrate-snapshot-optimizations/README.md). They are written for a programmer who has not worked on kernels, containers, or virtualization. Each note defines the idea, cites a primary source, and says where the idea shows up when changing Reprise.

Read them in order. Later notes assume the earlier ones.

1. [Rootfs, overlay, and tmpfs](01-rootfs-overlay-and-tmpfs.md). What the sandbox's `/` is, where writes go, and why "in memory" and "on disk" are different budgets.
2. [Page cache, holes, and hard links](02-page-cache-and-file-copies.md). Why the same snapshot file can restore quickly or slowly, and what a copy can silently change.
3. [The sandbox, the bundle, and runsc](03-the-sandbox-and-runsc.md). What gVisor actually runs, what an OCI bundle is, and why Docker is outside the measurement.
4. [Memory, checkpoint, and restore](04-memory-checkpoint-and-restore.md). Pages, zero pages, background restore, and the flags the runtime already exposes.
5. [How the host reaches the sandbox](05-reaching-the-sandbox.md). Network namespaces and veth pairs, and why the probe uses them.

Project terms (sandbox, instance, cold boot, restore, time to first response) stay in [`CONTEXT.md`](../../CONTEXT.md). These notes explain the operating-system words underneath them.

## What phase 1 actually touches

The plan measures local restore of one instance and then tries a short list of changes. The notes still explain neighboring ideas, because the research series uses them and a later optimization may need them. The last section of each note separates the two.

| You will change this soon | The idea lives in |
| --- | --- |
| The read-only directory a workload boots from, and `--overlay2=root:memory` | [Rootfs](01-rootfs-overlay-and-tmpfs.md) |
| Cold versus warm samples, `drop_caches`, `--direct` | [Page cache](02-page-cache-and-file-copies.md) |
| The OCI bundle, `runsc` arguments, systrap versus KVM | [Sandbox](03-the-sandbox-and-runsc.md) |
| `--exclude-committed-zero-pages`, `--compression`, `--background` | [Memory](04-memory-checkpoint-and-restore.md) |
| The network namespace and veth pair, including creating them before `restore` | [Network](05-reaching-the-sandbox.md) |
| Building the bundle while the snapshot files are read | [Memory](04-memory-checkpoint-and-restore.md), "Two preparations" |

These stay out of phase 1, and the notes say so when they come up: unpacking OCI layers, copying or hard-linking snapshot files, `fsync`, uploading snapshots, and the microVM mechanisms (a guest tmpfs, virtio-fs, Cloud Hypervisor demand paging).

## Which note unlocks which research post

| Post | Read first |
| --- | --- |
| [01 Share unpacked image layers](../research/substrate-snapshot-optimizations/01-share-unpacked-image-layers.md) | Rootfs |
| [02 Transfer memory extents](../research/substrate-snapshot-optimizations/02-transfer-memory-extents.md) | Memory, then page cache for the file representation |
| [03 Keep local copies sparse](../research/substrate-snapshot-optimizations/03-keep-local-copies-sparse.md) | Page cache |
| [04 Stage local snapshots with hard links](../research/substrate-snapshot-optimizations/04-stage-local-snapshots-with-hardlinks.md) | Page cache |
| [05 Overlap restore preparation](../research/substrate-snapshot-optimizations/05-overlap-restore-preparation.md) | Sandbox and memory |
| [06 Demand-page the working set](../research/substrate-snapshot-optimizations/06-demand-page-the-working-set.md) | Memory |
| [07 Move rootfs writes out of guest RAM](../research/substrate-snapshot-optimizations/07-move-rootfs-writes-out-of-guest-ram.md) | Rootfs |
| [08](../research/substrate-snapshot-optimizations/08-size-uploads-for-round-trips.md) and [09](../research/substrate-snapshot-optimizations/09-parallelize-the-whole-upload-pipeline.md), the upload pipeline | Skip until remote snapshots are in scope |
| [10 Remove fsync](../research/substrate-snapshot-optimizations/10-remove-fsync-from-reconstructible-state.md) | Page cache |
