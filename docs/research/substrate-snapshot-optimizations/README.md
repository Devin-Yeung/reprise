# Making snapshots fast: ten lessons from Substrate

A snapshot system is a pipeline: prepare a filesystem, capture execution state, move the bytes, rebuild the environment, and get the application doing useful work again. Its bottleneck moves as each stage improves.

These ten short posts retell engineering decisions from [Substrate](https://github.com/agent-substrate/substrate)'s commit history. The recurring surprise is that the expensive work often sits around the snapshot operation: unpacking an image again, copying empty memory, feeding concurrent uploads from one compressor, or flushing files that can be recreated.

An actor is Substrate's managed workload. A local **pause** keeps its checkpoint on the worker; a **suspend** publishes a snapshot to object storage. The runtime can be gVisor or a microVM; the memory-mode and host-rootfs stories below concern the microVM path.

1. [Stop unpacking the same image for every actor](01-share-unpacked-image-layers.md). Share prepared OCI layers instead of rebuilding identical filesystems.
2. [The memory image was mostly empty. Stop reading it.](02-transfer-memory-extents.md). Encode populated extents rather than compressing the whole logical address space.
3. [A harmless file copy turned 164 MiB into 2 GiB](03-keep-local-copies-sparse.md). Preserve sparsity through staging and repeated lifecycle transitions.
4. [The fastest local copy was no copy](04-stage-local-snapshots-with-hardlinks.md). Reuse immutable local files, then make future writers respect shared ownership.
5. [Restore was waiting for steps that could run together](05-overlap-restore-preparation.md). Move the join to the point where the outputs are actually needed.
6. [Restore the working set, until the hypervisor stops cooperating](06-demand-page-the-working-set.md). Use demand paging where it works, and adapt when dependency behavior changes.
7. [A package install should not consume the guest's RAM budget](07-move-rootfs-writes-out-of-guest-ram.md). Give filesystem growth a storage budget and capture its state explicitly.
8. [The small snapshot was waiting on round trips](08-size-uploads-for-round-trips.md). Tune request boundaries for the common object size.
9. [Eight upload streams cannot outrun one compressor](09-parallelize-the-whole-upload-pipeline.md). Parallelize production as well as transfer.
10. [A tiny metadata file waited behind another actor's checkpoint](10-remove-fsync-from-reconstructible-state.md). Reserve durability work for state that recovery actually needs.

Each post links the motivating commit and the relevant implementation at [revision `7317e083`](https://github.com/agent-substrate/substrate/tree/7317e083cf7a80ed26419eb14ba4f289a6846525). Performance numbers are the original engineers' reported measurements, with their workload context, rather than new benchmarks for this series.
