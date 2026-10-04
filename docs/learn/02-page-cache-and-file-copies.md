# Page cache, holes, and hard links

Restore reads a directory of files. Checkpoint writes one. Most of the surprise in that path is not the file's bytes. It is whether those bytes are already in RAM, whether the zeros take disk space, and whether two names point at one file.

## The page cache

When a process reads a file, the kernel keeps the file's pages in RAM. A later read of the same offsets can be served from that cache and skip the disk. The kernel calls this the page cache. Writing `1` to `/proc/sys/vm/drop_caches` drops clean page-cache pages. Writing `2` drops reclaimable slab objects such as dentries and inodes. Writing `3` drops both. Dirty pages stay. Running `sync` first turns dirty pages clean so more of them can be dropped. The kernel documentation says this is for testing and debugging: recreating the cache costs I/O and CPU, and the cache already shrinks on its own when memory is needed.[1]

A snapshot restore is a large read of the pages file. If that file is still in the page cache, the read is a memory copy. If it is not, the read waits on the disk. The plan measures both and treats the cold number as the headline. A cold sample writes `3` to `drop_caches` before it starts.[2]

`--direct` tells `runsc` to read or write the pages file with `O_DIRECT`. `open(2)` defines that flag as an attempt to minimize cache effects: the transfer goes directly between the userspace buffer and the file. It has alignment rules that depend on the filesystem, and it does not by itself give the durability guarantee of `O_SYNC`.[3] gVisor requires `--compression=none` for this flag, and the filesystem must support direct I/O. The documented case where it helps is a snapshot read once from disk and not restored again on the same machine, so caching it would be wasted RAM.[4]

On this benchmark the same snapshot is restored many times on one machine. Direct I/O and a warm page cache pull in opposite directions. The plan's "Direct I/O" row exists to measure that: the cold-versus-warm gap should shrink, and a warm run can get slower. `CheckpointOptions.direct_io` and `RestoreOptions.direct_io` are those two flags.[5]

### The workstation is a VM that reclaims this cache

WSL2's `memory` setting defaults to half of the Windows machine's RAM.[6] The plan raises it because a 7 GB guest is tight once two sandboxes and a page cache are in play.

`autoMemoryReclaim` defaults to `dropCache`, which reclaims cached memory immediately. `gradual` reclaims it slowly. `disabled` leaves it alone.[6] A warm-cache sample is only a warm-cache sample if nothing else drops the cache between the checkpoint and the restore. The plan sets `autoMemoryReclaim=disabled` for that reason.[2] Numbers from this VM are compared with each other. They are not bare-metal results.

## A hole is zeros that occupy no blocks

A file has a logical size, which is what `stat` reports as `st_size`, and an allocated size, which is the blocks the filesystem actually reserved. A **hole** is a run of bytes that reads back as zeros and, normally, has no blocks allocated. `lseek` with `SEEK_DATA` walks to the next allocated region. `SEEK_HOLE` walks to the next hole. A filesystem is allowed to report no holes at all, in which case `SEEK_HOLE` returns the end of the file and every zero looks like stored data.[7]

A copy that reads the file and writes the bytes allocates those zeros. The contents still compare equal. The allocated size does not. [Research 03](../research/substrate-snapshot-optimizations/03-keep-local-copies-sparse.md) is that bug: a 164 MiB snapshot became its full 2 GiB logical size, and later checkpoints stayed dense. The repair walks the allocated extents and copies those ranges, then sets the logical length so the trailing hole remains.

Phase 1 does not copy a snapshot from one directory to another. `runsc checkpoint` writes the directory that *is* the snapshot. You meet holes if a later change stages a copy, ships the file, or the next checkpoint reads back a file some other tool expanded. The check is allocated blocks as well as byte equality, then another checkpoint, because a dense file makes the next snapshot dense too.

## A hard link is a second name for one file

`link(2)` creates a new directory entry for an existing file. Both names are the file: same inode, same contents, same permissions. The new name has to be on the same filesystem. Neither name is the original.[8] The inode's link count is how many names it has. A write through any name is a write to the one file.

[Research 04](../research/substrate-snapshot-optimizations/04-stage-local-snapshots-with-hardlinks.md) used that to avoid copying a local snapshot into a restore directory: a second name, on the same filesystem, instead of a second copy of the bytes. The cost is ownership. A later write through the restore name would change the saved snapshot. The code that adopted hard links had to rename-replace files instead of truncating them, and had to copy when the link count said the file was shared.

Phase 1 has no staging directory, so it has no hard links. The idea matters as soon as one snapshot can be restored while something else might write the same inode: a second restore running concurrently, or a checkpoint that merges new pages into the previous file.

## `fsync` and `rename` answer different questions

`fsync` writes the file's modified cached data and its metadata to the storage device and waits until the device reports completion. It does not put the directory entry on disk. That takes a separate `fsync` of the directory.[9]

ext4's default journal mode is `data=ordered`: the data blocks that belong to a transaction are written to the filesystem before that transaction's metadata is committed to the journal. A transaction groups the metadata of the changes it contains, and the associated data is written first.[10] A small new file still needs a metadata commit. That commit can wait until other data in the same transaction has reached disk. [Research 10](../research/substrate-snapshot-optimizations/10-remove-fsync-from-reconstructible-state.md) is a pause benchmark stuck behind exactly that: `fsync` of a tiny metadata file waited for another actor's checkpoint.

Phase 1 does not call `fsync`. The plan accepts that a host crash can lose or corrupt a snapshot.[2] What it does protect is a reader observing a half-written snapshot. Checkpoint writes into `<id>.tmp/` and publishes it by renaming that directory to `<id>/`. `rename(2)` replaces an existing destination atomically: another process never observes the name missing in the middle of the replacement.[11] `Stage::Commit` is that publication. Atomic visibility to a reader and survival across a crash are two properties. The rename provides the first.

## Where you meet this

| When you are changing | The concept you need | What it decides |
| --- | --- | --- |
| The benchmark loop that the plan describes for `crates/reprise-bench` | Page cache, `drop_caches` | Each configuration runs cold and warm. Cold is the headline. Warm is meaningless if WSL reclaims the cache. |
| `CheckpointOptions.direct_io`, `RestoreOptions.direct_io` | `O_DIRECT` | Bypasses the page cache on the pages file. Requires an uncompressed snapshot. Compare against the cold/warm gap rather than turning it on by default. |
| `.wslconfig` on the workstation | `memory`, `autoMemoryReclaim` | RAM available to the guest, and whether a warm sample stays warm. |
| `Stage::Commit` and the `<id>.tmp` directory in the plan | `rename` versus `fsync` | Readers see a complete snapshot or none. A crash may still lose it. That is the phase 1 durability choice. |
| A later copy of snapshot files ([research 03](../research/substrate-snapshot-optimizations/03-keep-local-copies-sparse.md) and [04](../research/substrate-snapshot-optimizations/04-stage-local-snapshots-with-hardlinks.md)) | Holes, hard links | Copy allocated ranges only. A hard link shares the inode, so writers must not modify it in place. |
| A later durability requirement ([research 10](../research/substrate-snapshot-optimizations/10-remove-fsync-from-reconstructible-state.md)) | `fsync`, ext4 ordered mode | Sync the files whose loss a crash must not cause. A metadata `fsync` can wait behind unrelated data on the same filesystem. |

## Sources

1. [Documentation for `/proc/sys/vm/`](https://docs.kernel.org/admin-guide/sysctl/vm.html), section `drop_caches`.
2. [Snapshot runtime](../design/snapshot-runtime.md), "Measurement", "On disk", "Scope", and "Workstation setup".
3. [`open(2)`](https://man7.org/linux/man-pages/man2/open.2.html), `O_DIRECT` and `O_SYNC`.
4. [gVisor Checkpoint/Restore](https://gvisor.dev/docs/user_guide/checkpoint_restore/), "Direct I/O" and "Compression".
5. `CheckpointOptions` and `RestoreOptions` in `crates/reprise-runtime`.
6. [WSL configuration](https://learn.microsoft.com/en-us/windows/wsl/wsl-config), `.wslconfig` settings `memory` and `autoMemoryReclaim`.
7. [`lseek(2)`](https://man7.org/linux/man-pages/man2/lseek.2.html), `SEEK_DATA` and `SEEK_HOLE`.
8. [`link(2)`](https://man7.org/linux/man-pages/man2/link.2.html).
9. [`fsync(2)`](https://man7.org/linux/man-pages/man2/fsync.2.html).
10. [ext4 general information](https://docs.kernel.org/admin-guide/ext4.html), `data=ordered`.
11. [`rename(2)`](https://man7.org/linux/man-pages/man2/rename.2.html).

Next: [the sandbox, the bundle, and runsc](03-the-sandbox-and-runsc.md).
