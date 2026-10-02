# A package install should not consume the guest's RAM budget

The original microVM filesystem model put each container's writable rootfs layer on tmpfs inside the guest. It was convenient, and the written files naturally traveled with the memory snapshot. But every written byte consumed guest RAM.

That assumption breaks for development workloads. Installing packages or producing build artifacts can write hundreds of megabytes without needing an equally large process working set. The [host-backed rootfs commit](https://github.com/agent-substrate/substrate/commit/c1339e5f020190e4d124007279b18bf4ffc5af5b) reports an effective tmpfs limit of roughly **264 MiB on the standard 2 GiB guest**, after other users of `/run` were counted. Write-heavy actors hit `ENOSPC`; less demanding actors still consumed memory that could have supported more guests.

## Put filesystem growth in the filesystem

Substrate moved overlay assembly to the host. Read-only OCI layers form the lower; per-actor host directories hold the upper and workdir. The merged filesystem reaches the guest through the existing virtio-fs share. Rootfs writes now consume host storage and cache rather than residing in guest tmpfs.

That makes filesystem capture explicit. A full checkpoint archives the writable upper while the guest is paused, concurrently with memory capture. Restore extracts it alongside bundle preparation. The image supplies the immutable lower, so the snapshot carries the actor's changes rather than another copy of the base image.

The tricky part was preserving deletions. OverlayFS whiteouts and opaque-directory attributes encode filesystem state just as surely as file contents do. Omitting them would make deleted files reappear after resume. The change added archive support for those records and disabled overlay features that could leave stale references to lower-layer inodes after reconstruction.

A [follow-up](https://github.com/agent-substrate/substrate/commit/ad830889adcb0fabc8553d9bc83ad8c7d395ab6d) also removed workdirs from the archive. OverlayFS rebuilds them under the pinned mount configuration, and an interrupted copy-up can leave a file-sized temporary object there. Saving that temporary state bought nothing.

The introducing commit reports suspend/restore latency at parity across 25 full-suspend cycles per side on GKE. The main win was eliminating the guest-RAM cap and improving density without adding a measured latency regression in that comparison.

**Takeaway:** Match resource accounting to the workload. Filesystem growth should have a storage budget; making it consume the guest's memory budget can make a small process unexpectedly expensive or unable to run.

Implementation: [upper-layer capture](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/cmd/ateom-microvm/rootfsupper.go), [host overlay](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/cmd/ateom-microvm/internal/kata/overlay_linux.go).

[Back to the series](README.md)
