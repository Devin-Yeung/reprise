# Rootfs, overlay, and tmpfs

A sandbox has a filesystem. The program inside it opens `/bin/memory-state` and writes `/tmp/state`, and those paths have to land somewhere on the host. Where they land decides two things you will measure: how much memory a write costs, and whether that write is still there after restore.

## The directory the process calls `/`

Every process resolves a path that starts with `/` from its root directory. For a container, that directory is prepared on the host and then made the process's `/`. That directory is the **rootfs**.

In this repository the field is `Workload.rootfs`: a self-contained directory. Nothing inside it may point outside it, for example into `/nix/store`, because the sandbox will not have that store. The directory stays read-only. File writes are kept in memory, so a snapshot includes them.[1]

The Linux kernel uses the same word for something else: during boot it mounts a special ramfs and calls that mount `rootfs`.[2] In these notes, and in the runtime, the word means the sandbox's root directory.

## An image, a rootfs, and a bundle are three artifacts

An OCI **image** is a stack of tar archives called layers. Each layer is a changeset: added and modified files, plus records of deletions. Applying the layers in order produces a directory tree. That directory is a rootfs. It is not the image.[3]

A **bundle** is what an OCI runtime such as `runsc` actually executes: a `config.json` plus the rootfs directory that `config.json` names. The runtime does not pull images and does not unpack layers.[4] gVisor's own quick start builds that directory first (in the example, by exporting a Docker image), writes `config.json` with `runsc spec`, and only then calls `runsc run`.[5]

Phase 1 does the same split. Nix is how the rootfs directory gets built. Unpacking images, sharing unpacked layers between sandboxes, and caching those layers are the work in [research 01](../research/substrate-snapshot-optimizations/01-share-unpacked-image-layers.md), and the plan leaves that until restore itself is measured.[1]

### Deletions have to be recorded

A layer that only contains the files it adds cannot say "this file from the layer below is gone." OCI records that with a **whiteout**: an empty file whose name is `.wh.` plus the basename of the deleted path. A file named `.wh..wh..opq` is an opaque whiteout: hide every child of this directory from the layers below.[3]

The kernel's overlay filesystem records the same idea in a different encoding, described in the next section. You meet OCI whiteouts when you unpack an image yourself. You meet the kernel encoding when you archive a host overlay's upper directory. Phase 1 does neither. The correctness gate still writes one file and requires it to exist after restore, which is the easy case of "the upper layer survived."

## Overlay: one tree made of two

An overlay filesystem shows a single tree that is the combination of an **upper** directory and a **lower** directory. A name that exists in both is taken from the upper. The lower can be read-only, so many sandboxes can share one image directory. Writes go to the upper. A third directory, the **workdir**, must be empty and on the same filesystem as the upper; the kernel uses it as scratch space while it builds the merged view.[6]

```text
mount -t overlay overlay \
  -o lowerdir=/lower,upperdir=/upper,workdir=/work \
  /merged
```

Changing a file that currently lives only in the lower layer copies it into the upper layer first. The lower file stays as it was. The gVisor write-up calls those copies "copied-up" files: they, and newly created files, are the ones the upper layer holds.[7]

Deleting is the subtle part. The lower directory must not change, so the upper layer records the deletion:

- A **whiteout** hides one lower name. The kernel creates it as a character device with device number 0/0, or as a zero-length file with the `trusted.overlay.whiteout` extended attribute. The whiteout itself is hidden from the merged view.
- An **opaque directory** hides a whole lower directory. The kernel sets the extended attribute `trusted.overlay.opaque` to `y` on the upper directory.[6]

Those bytes are not the OCI `.wh.` files. A tar of an upper directory that drops the character devices and the extended attributes will restore deleted files, because the lower layer still has them. That is the bug [research 07](../research/substrate-snapshot-optimizations/07-move-rootfs-writes-out-of-guest-ram.md) had to close on the microVM path.

Docker and Kubernetes usually build this overlay on the host. The lower layers are the image. The upper layer is the container's private writes, and it is thrown away when the container is destroyed.[7]

## tmpfs keeps files in memory

**tmpfs** is a Linux filesystem that stores every file in virtual memory. Unmounting it drops the files. Nothing is written to a disk by the filesystem itself. It can grow and shrink with its contents, and it can move idle pages to swap when swap is enabled. With the default mount options its size limit is half of physical RAM. The pages show up as `Shmem` in `/proc/meminfo`.[8]

A write that the application thinks is a file write is, inside tmpfs, an allocation. Filling the size limit fails with `ENOSPC`. That is a filesystem error caused by a memory budget.

## gVisor puts the upper layer inside the sandbox

gVisor serves host files through a helper process, the gofer. A write on the host overlay used to mean a round trip to that process and a host system call. The upper layer dies with the container, so gVisor moved it inside the sandbox: a tmpfs implemented by the Sentry, with the read-only lower layer still served from the host. Writes to that tmpfs stay off the gofer path.[7]

That internal tmpfs is not a host `mount -t tmpfs`. It is gVisor's own filesystem. The `--overlay2` flag chooses what backs its file bytes. The flag's form is `{mount}:{medium}`, where `mount` is `root` (the rootfs only) or `all`, and `medium` is one of:[9]

| Medium | Where the file bytes live | What that costs |
| --- | --- | --- |
| `memory` | The sandbox's own memory | Fast writes. Large writes consume the sandbox memory budget and can exhaust it.[7] |
| `self` | A filestore file created inside the host rootfs. A whiteout hides that file from the application. | The host directory must be writable. Kubernetes can see the usage by stating that file.[7] |
| `dir=/absolute/path` | A filestore created in that host directory | The bytes leave the sandbox memory without modifying the rootfs.[9] |
| `none` | No internal overlay. Changes go through to the host filesystem. | Host-visible writes, and the gofer is on the path again.[7][9] |

`root:self` is the default.[9] An optional `,size=` sets the tmpfs size limit; omitted, gVisor uses its own default. The flag comment does not publish that default as a number.[9]

### Why Reprise uses `root:memory`

The rootfs directory stays read-only, so `root:self` cannot create its filestore inside it. The plan therefore passes `--overlay2=root:memory`. Writes live in sandbox memory and travel with the memory snapshot, in the same way other memory does.[1]

That is the gVisor form of the problem [research 07](../research/substrate-snapshot-optimizations/07-move-rootfs-writes-out-of-guest-ram.md) describes for a microVM. There, the writable layer was a tmpfs inside the guest, so a package install consumed guest RAM (the commit reports an effective limit of about 264 MiB on a 2 GiB guest) and failed with `ENOSPC`. Substrate moved the overlay onto the host and showed the merged tree to the guest with virtio-fs, a protocol for using a host directory from a virtual machine.[10] Reprise has no guest kernel, so it has no virtio-fs. Phase 1 accepts the memory medium because the planned file write is one small file in the correctness gate, while W2's large allocations are anonymous memory, covered in the memory note. A later workload that installs packages or writes build output into `/` is the point to revisit `root:memory`.

### What a checkpoint includes

Two gVisor features save filesystem state, and they do not have the same requirements.

`runsc checkpoint` saves the sandbox so `runsc restore` can continue it. Ayush Ranjan, who wrote the rootfs overlay, answered a request to extract the upper layer by saying checkpoint/restore should already work with every `--overlay2` mode.[11] This repository has not confirmed that on the workstation. The correctness gate is the check: cold boot, write a file, checkpoint, restore, and require the file to still be there. The plan lists that as an open question to answer by running it.[1]

`runsc fscheckpoint` is a separate command. It saves rootfs changes into their own snapshot. Its documented prerequisite is an overlay whose upper tmpfs is disk-backed, which is what the default `--overlay2` does. tmpfs mounts backed by the application's main memory file are excluded.[12] `root:memory` is that excluded case. Phase 1 does not call `fscheckpoint`. If a later design wants a filesystem snapshot that is separate from the memory image, the medium has to be a disk-backed one (`self` or `dir=`), and the rootfs has to be writable or the filestore has to live elsewhere.

## Where you meet this

| When you are changing | The concept you need | What it decides |
| --- | --- | --- |
| `Workload.rootfs` in `crates/reprise-runtime`, and the Nix derivation that fills that directory | Rootfs | The directory must be complete and contain no links to the outside. `Stage::Bundle` writes the `config.json` that points `runsc` at it. |
| The `runsc` arguments for cold boot | `--overlay2=root:memory` | Writes stay off the read-only directory and inside sandbox memory. `root:self` needs a writable rootfs. |
| The correctness gate in the plan (write a file, restore, read it back) | Upper layer, and what `runsc checkpoint` saves | Confirms that a memory-backed write survives restore. A failure here is about the overlay medium, before any performance flag. |
| A later image-preparation chapter ([research 01](../research/substrate-snapshot-optimizations/01-share-unpacked-image-layers.md)) | OCI layers and `.wh.` whiteouts | Unpacking must apply deletions. Sharing one unpacked lower directory across sandboxes is safe because the lower stays read-only. |
| A later write-heavy workload | tmpfs size, `memory` versus `self` or `dir=` | A package install in `root:memory` spends the sandbox's RAM. That is the research 07 tradeoff, on gVisor. |

## Sources

1. [Snapshot runtime](../design/snapshot-runtime.md), sections "Concepts", "Rootfs and network", "Correctness gate", and "Open questions". The crate field is `Workload` in `crates/reprise-runtime/src/runtime.rs`.
2. [ramfs, rootfs and initramfs](https://docs.kernel.org/filesystems/ramfs-rootfs-initramfs.html). The kernel's boot `rootfs`.
3. [OCI Image Spec: Image Layer Filesystem Changeset](https://github.com/opencontainers/image-spec/blob/main/layer.md). Layers, whiteouts, opaque whiteouts.
4. [OCI Runtime Spec: Filesystem Bundle](https://github.com/opencontainers/runtime-spec/blob/main/bundle.md).
5. [gVisor OCI Quick Start](https://gvisor.dev/docs/user_guide/quick_start/oci/).
6. [Overlay Filesystem](https://docs.kernel.org/filesystems/overlayfs.html). Upper and lower, whiteouts, opaque directories.
7. Ayush Ranjan, [Rootfs Overlay](https://gvisor.dev/blog/2023/05/08/rootfs-overlay/), gVisor blog, 8 May 2023. Also published as the [Google Open Source Blog post](https://opensource.googleblog.com/2023/04/gvisor-improves-performance-with-root-filesystem-overlay.html).
8. [Tmpfs](https://docs.kernel.org/filesystems/tmpfs.html).
9. [`--overlay2` flag definition](https://github.com/google/gvisor/blob/master/runsc/config/flags.go) and [`Overlay2` in `runsc/config/config.go`](https://github.com/google/gvisor/blob/master/runsc/config/config.go). Live `master`; confirm against the `runsc` version installed on the workstation (`runsc --help`).
10. [virtiofs](https://docs.kernel.org/filesystems/virtiofs.html). Used by the microVM path in research 07, not by Reprise.
11. [google/gvisor issue 11892](https://github.com/google/gvisor/issues/11892), comment by ayushr2 on 7 July 2025. A comment, not a test on this machine.
12. [gVisor Filesystem Snapshots](https://gvisor.dev/docs/user_guide/fs_snapshot/).

Next: [page cache, holes, and hard links](02-page-cache-and-file-copies.md).
