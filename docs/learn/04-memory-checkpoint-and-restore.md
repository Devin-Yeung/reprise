# Memory, checkpoint, and restore

Cold boot runs the program from the beginning. Restore skips that and loads a saved Sentry, including the application's memory. The time you can take out of restore is mostly the time spent moving pages the program does not touch before it answers.

## A page, a zero page, and the working set

Linux allocates memory in **pages**. `getpagesize(2)` returns the size of that unit for the running kernel; on the workstation's x86_64 kernel it is ordinarily 4 KiB.[1] gVisor saves and skips memory in the same unit.

A **committed zero page** is a page the process has allocated whose bytes are all zeros. Allocating a large buffer and not filling it produces a lot of these. `--exclude-committed-zero-pages` tells `runsc checkpoint` to skip saving them. The snapshot gets smaller and restore gets faster. Checkpoint can get slower, because every committed page is scanned to see whether it is zero. gVisor calls out large zero-filled regions, such as model weights that have not been filled, as the case where this pays.[2] `CheckpointOptions.exclude_zero_pages` is that flag.[3]

W2 in the plan is this experiment with the dials visible: an N MiB allocation, filled with zeros or with random bytes, N in {64, 256, 1024}. Zeros are where exclusion matters. Random bytes make cold boot do real work, so the same workload also carries the restore-versus-cold-boot comparison.[4]

The **working set** here is operational: the pages the program touches between the moment it is allowed to run and the moment it first answers. gVisor's background restore is aimed at that interval. It calls the resulting latency "time to first instruction."[2] The classic term is Denning's working set, the pages referenced in a recent window.[5] You do not need the formal model. You need the inequality: if the working set is much smaller than the allocated address space, loading every page before the program runs wastes the wait.

## What `runsc checkpoint` writes

`runsc checkpoint --image-path=<dir>` writes the checkpoint files into that directory. The directory must be unique. By default the container's processes stop.[2] This repository calls the directory a snapshot. gVisor's flag name is image path. [`CONTEXT.md`](../../CONTEXT.md) keeps those words apart: an image is the OCI image a cold boot starts from, a snapshot is the saved state.

`Stage::RunscCheckpoint` is the `runsc` invocation. `Stage::Commit` writes the manifest (runsc version, platform, checkpoint options, network addresses) and renames the directory into place, so a directory without the temporary suffix is complete. The file list is read from the directory rather than hardcoded, because gVisor decides which files it writes.[4]

Restore is a new container. `runsc create` then `runsc restore --image-path=<dir>`. Every top-level `runsc` flag used at create time has to be passed again. The `runsc` binary has to be the one that wrote the snapshot. The destination machine has to have the CPU features that were enabled when the snapshot was taken; gVisor checks.[2] `runsc cpu-features` lists the features on the current machine. An annotation, `dev.gvisor.internal.cpufeatures`, can pin the set so two machines with different CPUs can still exchange a snapshot.[2] Phase 1 restores on the same workstation, so the pin is unused, and the check is still why a snapshot records its platform and `runsc` version.

`--leave-running` checkpoints and then immediately restores so the same container id keeps running. That is a different operation from suspend. The plan's suspend ends the instance.

### Compression and what it rules out

`--compression=none` is the default. The snapshot is several files, and gVisor restores kernel state and memory in parallel. `--compression=flate-best-speed` spends CPU to shrink the files. Background restore and direct I/O both require `none`.[2] `CheckpointOptions.compress` selects `flate-best-speed`. Turning it on means the background and direct-I/O experiments are off. The plan's "Compression" row is that trade: size against speed.[3][4]

## Background restore

`--background` on `runsc restore` lets the program run as soon as the kernel state is loaded. The rest of the memory and the file data are read while it runs. If the program touches a page that is not loaded yet, gVisor loads that page before continuing the thread. The snapshot has to be uncompressed. After `runsc restore` returns, the sandbox may still hold the pages file open, so deleting it does not free the disk space until the background load finishes. `runsc wait --restore` waits for that.[2]

`RestoreOptions.background` is the flag.[3] It is the plan's first candidate for W2's time to first response, because W2's address space is large and the HTTP handler's working set is not.[4]

This is demand paging applied to restore. In ordinary virtual memory, a page can be part of the address space before it occupies RAM; the access faults, and the kernel fills the page. Background restore is gVisor doing the analogous thing for pages that live in a snapshot file.

[Research 06](../research/substrate-snapshot-optimizations/06-demand-page-the-working-set.md) is the same goal on a microVM, implemented by Cloud Hypervisor, and it is not a `runsc` flag. The post's lesson still applies: the optimization depends on the hypervisor actually faulting pages in on demand. A later Cloud Hypervisor release prefaulted them in the background and the guest never became ready. If a `runsc` upgrade changes when `--background` loads pages, the W2 result has to be remeasured. Phase 1 does not implement userfaultfd or a guest page-fault handler.

## Two preparations, then one join

A restore needs a bundle and a snapshot directory. Building the bundle does not read the snapshot. Reading the snapshot does not need the finished bundle. `runsc restore` needs both. Doing the two one after another adds their times for no dependency.

The public API stays blocking: `Runtime::restore` returns when `runsc restore` has returned. Inside the library, the plan allows the bundle build and a prefetch of the snapshot files to run together, joined before `runsc restore`.[4] That is [research 05](../research/substrate-snapshot-optimizations/05-overlap-restore-preparation.md) on a single machine, with no object-storage download. It shows up on W1, where there is little memory to move and the fixed work dominates. The stage names `bundle` and `runsc.restore` are how you see whether the overlap landed.

## What the benchmark calls ready

Time to first response starts when `cold_boot` or `restore` is called and stops at the first successful `GET /state` from the host.[4] `runsc restore` returning is earlier than that. The wait after it is the `ready` span. Background restore is a bet that `ready` shrinks even though pages are still loading, because the handler only needs its working set.

W1 is a few megabytes and is also the correctness gate: the boot nonce, the stored value, and the revision survive, the written file is still there, and the next mutation gets the next revision. An optimization that fails that gate does not count, whatever it does to the histogram.[4]

The discussion's lower bound, a sandbox frozen in RAM with no serialization, is not a phase 1 measurement. It would still occupy RAM for every frozen sandbox. The thing under study is the snapshot, which lets the instance end.

Remote copies of the snapshot, and the extent-encoding and upload work in research 02, 08, and 09, are out of phase 1. "Skip the zero pages" is the local version of the same instinct: do not move bytes the program does not need.

## Where you meet this

| When you are changing | The concept you need | What it decides |
| --- | --- | --- |
| `Instance::checkpoint` and `Snapshot` | Checkpoint directory, image path, commit | What `runsc` wrote, plus the manifest, published by rename. Version and platform are part of compatibility. |
| `CheckpointOptions.exclude_zero_pages` | Committed zero pages | Snapshot size and restore time on W2's zero ballast. Checkpoint time may rise. |
| `CheckpointOptions.compress` | `flate-best-speed` versus several uncompressed files | Smaller files. Disables background restore and direct I/O. |
| `RestoreOptions.background` | Background restore, working set | TTFR on W2. Keep the snapshot files until the load finishes. Uncompressed only. |
| The benchmark's `ready` span versus `runsc.restore` | Time to first response | The flag succeeds when the first HTTP response moves, not when `runsc` returns. |
| Overlapped bundle build and snapshot prefetch | Independent preparation | Fixed overhead on W1. Join before `runsc restore`. |
| Reading [research 06](../research/substrate-snapshot-optimizations/06-demand-page-the-working-set.md) | Demand paging on a microVM | The gVisor counterpart is `--background`. The failure mode to remember is a dependency that starts loading every page anyway. |

## Sources

1. [`getpagesize(2)`](https://man7.org/linux/man-pages/man2/getpagesize.2.html).
2. [gVisor Checkpoint/Restore](https://gvisor.dev/docs/user_guide/checkpoint_restore/). Usage, compression, zero pages, direct I/O, background restore, CPU features.
3. `CheckpointOptions` and `RestoreOptions` in `crates/reprise-runtime`.
4. [Snapshot runtime](../design/snapshot-runtime.md), "On disk", "Measurement", "Workloads", "Optimization candidates", and "Correctness gate".
5. Peter J. Denning, "The working set model for program behavior," *Communications of the ACM* 11, no. 5 (1968). [doi:10.1145/363095.363141](https://dl.acm.org/doi/10.1145/363095.363141). The formal model. The operational definition used here is gVisor's, in source 2.

Next: [how the host reaches the sandbox](05-reaching-the-sandbox.md).
