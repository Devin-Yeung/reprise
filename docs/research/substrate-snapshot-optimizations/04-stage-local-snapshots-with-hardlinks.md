# The fastest local copy was no copy

Preserving holes fixed the worst local-copy amplification, but staging still rewrote the populated working set. In a frequent pause/resume loop, that meant paying for the same memory bytes again and again even though the checkpoint was already on the right node.

Substrate changed local restore staging to create hardlinks. The saved checkpoint and the restore directory sit under the same actor directory, so the common path needs another name for an existing inode rather than another copy of its contents. Crossing a filesystem boundary falls back to a sparse copy.

The [commit containing the change](https://github.com/agent-substrate/substrate/commit/d3aec58cddfaf52d0c9a96dbb77b5e2b1ccbb08e) separates its effects from a subsequent fsync optimization. In a 120-second, single-node pause/resume benchmark with 15 `glutton` actors, 512 MiB RAM capacity, and 32 MiB churn per cycle, the hardlink step raised throughput from **53.3 to 80.0 cycles per minute**. Median `ResumeActor` latency fell from **6.0 to 4.1 seconds**. There was still another bottleneck, but eliminating the copy made a measurable difference.

## Sharing changes the ownership rules

A hardlink is not an independent file. A later write through the staging name would also change the saved checkpoint. The optimization therefore reaches beyond the staging helper.

Cloud Hypervisor reads the staged memory image for demand paging. Configuration rewrites replace the file via rename instead of truncating the shared inode. When the next checkpoint merges new memory pages into a restore base, it checks the link count: a shared base takes the copying merge path rather than being modified in place.

That last rule is the tradeoff. Hardlinks remove restore copying, but a later merge may need a private copy to protect the old restore point. The system chooses based on ownership at the moment of mutation.

**Takeaway:** Before optimizing a copy, ask why two independent copies are needed. If the data can stay immutable, names may be enough. Then audit every future writer, because sharing changes correctness obligations across the lifecycle.

Implementation: [local staging](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/cmd/atelet/main.go#L1389-L1473), [shared-base protection](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/cmd/ateom-microvm/internal/ch/merge.go).

[Back to the series](README.md)
