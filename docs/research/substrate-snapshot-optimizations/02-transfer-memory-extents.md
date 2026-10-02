# The memory image was mostly empty. Stop reading it.

A 2 GiB guest does not necessarily have 2 GiB of useful snapshot data. Its memory image can be a sparse file: a large logical address space with only a small fraction backed by allocated file extents.

A conventional compression pipeline misses that distinction. Reading a hole returns zeroes, so the compressor still scans the whole logical file. The output may be small, but producing it has already spent CPU and read bandwidth on empty space. “Zeroes compress well” answers the storage question and leaves the processing question open.

The [sparse-transfer commit](https://github.com/agent-substrate/substrate/commit/9360791710e4ce41fc8e79402ad79873365fef2d) identifies mostly free guest RAM as the reason to change the representation. Substrate now walks populated extents with `SEEK_DATA` and `SEEK_HOLE`, recording each extent's offset and length alongside its data. Only that data enters compression. The decoder writes extents at their original offsets and restores the file's logical length, leaving gaps sparse.

## Make absence part of the format

The key move is to represent holes explicitly instead of turning them into a stream of zero bytes. Work then follows populated extents rather than guest capacity. The source comments illustrate the difference as scanning roughly 150 MiB rather than a 2 GiB logical image.

The format is streamable: extent records arrive incrementally and an end marker closes the stream, so the writer does not need a complete extent table before starting the upload. A magic value and version distinguish it from older plain-zstd snapshots, which remain readable.

This representation preserves filesystem sparsity, not a history of page changes. An allocated extent may contain zeroes, and an extent is not necessarily a dirty page. Those distinctions matter because this remains a self-contained snapshot, with no dependency on an earlier snapshot to decode it.

**Takeaway:** Before improving compression, ask whether the bytes need to enter the compressor at all. Logical size, populated size, and compressed size describe different costs.

Implementation: [extent format](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/pkg/objectstorage/sparsezstd.go), [format dispatch and decoding](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/pkg/objectstorage/objects.go).

[Back to the series](README.md)
