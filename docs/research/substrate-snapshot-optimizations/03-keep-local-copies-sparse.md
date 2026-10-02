# A harmless file copy turned 164 MiB into 2 GiB

The snapshot was sparse when it was captured. The wire format preserved that sparsity. Then a local staging copy quietly undid the work.

Substrate used `io.Copy` to stage a local checkpoint for restore. That preserves file contents, but it reads holes as zeroes and writes those zeroes as allocated data. The [fixing commit](https://github.com/agent-substrate/substrate/commit/285232a960df65c262e93aaee314b4b228dd23fb) describes a **164 MiB snapshot expanding to its full 2 GiB logical size** during this step.

The damage compounded across lifecycle transitions. Cloud Hypervisor then loaded the expanded image, and the next checkpoint became dense too. Five pause cycles on one actor left **8.2 GB of local checkpoints**, where a few hundred MiB would have sufficed.

## Preserve the representation through every handoff

The replacement copy walks source extents and transfers only populated ranges. It then sets the destination's logical length, including any trailing hole. Where available, kernel range copying moves those ranges without shuttling all the data through a userspace buffer. Filesystems that cannot report holes fall back to a dense copy.

This is an instructive bug because an ordinary byte-for-byte comparison would pass. The zeroes are correct. The regression lives in disk allocation and in what the next runtime does with the resulting file. A useful round-trip check therefore needs both content equality and allocated-block measurements, followed by another checkpoint cycle.

The improvement also explains why local restore deserves its own analysis. Avoiding object storage does not automatically make a path cheap: a local full-file rewrite can still dominate a pause/resume loop.

**Takeaway:** Physical representation can be part of a performance contract. If one helper destroys sparsity, every later stage may inherit the larger workload, even though the file's bytes remain correct.

Implementation: [sparse copy](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/cmd/atelet/internal/sparsefile/sparsefile.go), [kernel range-copy path](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/cmd/atelet/internal/sparsefile/copyrange_linux.go).

[Back to the series](README.md)
