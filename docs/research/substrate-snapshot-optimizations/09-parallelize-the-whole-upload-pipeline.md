# Eight upload streams cannot outrun one compressor

Larger snapshots needed concurrent uploads. S3 offered a multipart upload manager; GCS required a compose-based implementation. Splitting the output stream into parts seemed like the obvious way to keep several connections busy.

It exposed the next bottleneck. All those connections still waited for one compressor to produce their input.

The [parallel-upload commit](https://github.com/agent-substrate/substrate/commit/37741c0b64c2ace85f657654dd60a8c3d53dd521) describes discovering that compression constrained the new uploader. The current implementation's comments explain why simply increasing zstd encoder concurrency was insufficient: its streaming encoder still compressed one block at a time, with other work overlapping around that block. More consumers did not make the producer fast enough.

## Start parallelism where the bytes are produced

Substrate added two forms of parallelism. Its streaming sparse writer divides input into independent 8 MiB zstd frames, encodes them on separate workers, and emits them in order. A normal zstd reader can decode the concatenated frames. This sacrifices cross-frame compression matches in exchange for actual multicore encoding.

For large sparse GCS files, the implementation goes further: partition populated source ranges, then let each part independently read, compress, and upload over its own connection. GCS composes the parts in order into one decodable object. This fans out the producer as well as the network stage. Each range encoder stays single-threaded because the outer pipeline already supplies concurrency.

The commit reports a **451 MB working set dropping from 4.66 to 1.51 seconds** after both changes, with almost unchanged output size. A tiny full-snapshot counter workload improved from **0.79 to 0.62 seconds** even though it stayed below the upload-chunk threshold. That smaller case is useful evidence: compression parallelism could help independently of network fan-out.

Part counts and buffers are bounded, and small files use the simpler streaming path. Otherwise, many concurrent actors could trade a latency win for excessive node memory consumption.

**Takeaway:** Parallelize the bottleneck that produces the work. Adding concurrency downstream achieves little when every consumer waits on the same serial stage.

Implementation: [parallel frames](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/pkg/objectstorage/parzstd.go), [range-based upload pipelines](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/pkg/objectstorage/gcssparse.go), [part planning](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/pkg/objectstorage/sparseparts.go).

[Back to the series](README.md)
