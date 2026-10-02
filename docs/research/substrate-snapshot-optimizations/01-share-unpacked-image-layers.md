# Stop unpacking the same image for every actor

A snapshot can restore a process quickly and still deliver a slow resume. Before the process runs, someone has to build its filesystem. If that means downloading and unpacking the same container image again, memory restore is only one small part of the wait.

Substrate's image-cache change tackled exactly this repeated work. The old path built a fresh root filesystem per run. The new path keeps unpacked OCI layers in a content-addressed pool on each node. Actors mount those layers as read-only OverlayFS lowers and get their own writable upper. Starting another actor from the same image becomes filesystem assembly rather than another full extraction.

## Cache the expensive result

The useful cache entry is the unpacked layer, not just the downloaded archive. Caching compressed bytes would avoid a network transfer while leaving decompression and filesystem creation on every actor's critical path. Sharing unpacked layers removes those costs too, and content addressing lets different images reuse identical layers.

The implementation also splits responsibilities at the privilege boundary: the unprivileged worker pulls and unpacks; the privileged runtime finalizes whiteouts and mounts the overlay. The cache survives worker restarts, and streaming pulls keep memory proportional to buffers rather than the image size.

The [introducing commit](https://github.com/agent-substrate/substrate/commit/610e3916ad118c36784bf0c9820fa22c8af2e52b) reports the OCI-unpack phase at roughly **3 ms versus 15–20 seconds** in its counter-demo suspend/resume validation. That is a phase measurement, with reusable layers available. The same change was exercised against 411 SWE-bench-scale images, which makes the motivation more concrete than a small demo alone: image preparation matters for real development environments.

Sharing makes lifecycle management part of the design. A layer cannot be evicted while an actor uses it; the later cache implementation tracks live references and retires unused layers.

**Takeaway:** Cache the output of expensive preparation. When identical environments start repeatedly, a shared immutable filesystem can save more startup time than another improvement to memory restore.

Implementation: [layer cache](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/internal/imagecache/imagecache.go), [overlay assembly](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/internal/imagecache/bundle_linux.go), [eviction](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/internal/imagecache/gc.go).

[Back to the series](README.md)
