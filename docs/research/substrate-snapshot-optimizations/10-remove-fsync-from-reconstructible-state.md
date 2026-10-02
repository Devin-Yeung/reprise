# A tiny metadata file waited behind another actor's checkpoint

After eliminating local snapshot copies, resume was still surprisingly slow. The remaining bottleneck was not a large memory image. It was writing small metadata files.

During a pause benchmark, the author of [the fsync-removal change](https://github.com/agent-substrate/substrate/commit/d3aec58cddfaf52d0c9a96dbb77b5e2b1ccbb08e) traced the delay to `writeFileAtomic()` calls for `sandbox-assets.json` and system-info volumes. These new files required ext4 metadata updates. With the filesystem in ordered mode, syncing the file and directory could wait for other pending writes, including checkpoints from other actors.

A tiny write had become a synchronization point for a much larger shared workload.

## Ask what must survive a node crash

System-info files can be recreated. The sandbox-assets manifest is written during run or resume and read during checkpoint; once checkpoint finishes, the snapshot contains its own manifest. If the node fails before that checkpoint, the running actor is already lost. Forcing these intermediate files to durable storage did not provide an additional recoverable state.

Substrate kept the file-publication behavior it needed and removed the unnecessary fsyncs. Atomic visibility to a reader and durability after a machine crash are separate requirements; the helper had been providing both where only the first was useful.

The commit's single-node test used 15 `glutton` actors, 512 MiB RAM capacity, 32 MiB churn per cycle, and pause-only operation over 120 seconds. After the hardlink change, median resume was **4.1 seconds**. Removing these fsyncs brought it to **210 ms**. Throughput rose from **80.0 to 106.3 cycles per minute**. Relative to the original baseline, the combined changes doubled throughput; the staged comparison identifies where the remaining latency went.

This is also why testing one idle actor would miss much of the story. The expensive interaction appeared when one actor's metadata operation encountered other actors' checkpoint traffic on the same filesystem.

**Takeaway:** Put durability at the recovery boundary. Syncing reconstructible intermediate state can couple unrelated workloads and dominate tail latency without improving recovery.

Implementation: [sandbox manifest writes](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/cmd/atelet/main.go), [system-info publication](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/cmd/atelet/systeminfovolume.go).

[Back to the series](README.md)
