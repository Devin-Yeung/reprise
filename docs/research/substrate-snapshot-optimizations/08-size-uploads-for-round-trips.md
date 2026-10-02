# The small snapshot was waiting on round trips

Snapshot upload looked like a bandwidth problem. For the common idle microVM snapshot, it was mostly a request-count problem.

The [GCS chunk-size commit](https://github.com/agent-substrate/substrate/commit/a0523de4e7e81a70734511b32665b3072d2d9682) describes an upload taking roughly 800 ms of a 1.65-second server-side bake. A typical compressed golden snapshot was about 24 MiB, while the GCS client's resumable-upload default chunk was 16 MiB. That modest object crossed a chunk boundary and paid for two sequential round trips.

Substrate increased the writer's chunk size to 64 MiB, fitting that common snapshot into one request.

## Two sizes, two bottlenecks

The author tested the exact streaming upload path from a GKE `c3-standard-4` worker in `us-central1-f`, using the snapshot bucket. Three 24 MiB uploads took **425–530 ms with 16 MiB chunks** and **258–314 ms with 64 MiB chunks**. The commit summarizes this as about a 35% reduction in upload time.

Bigger was not uniformly better: the 128 MiB setting produced worse timings in those runs. Chunk size is also a buffering decision, so maximizing it is not a free policy when many actors upload concurrently.

The larger-object experiment exposed a different limit. At 300 MiB, increasing chunk sizes from 16 to 128 MiB left single-stream throughput in roughly the same band. Four parallel composite parts reached 233–257 MiB/s in the reported measurements. Once transfer time dominated, eliminating one round trip was no longer the useful lever.

This is why “optimize snapshot upload” cannot be one setting. A small compressed golden image and a large active guest image occupy different operating regimes. The former benefits from fewer sequential requests; the latter needs a pipeline capable of feeding multiple transfers.

**Takeaway:** Measure the workload's actual object sizes. When latency comes from request boundaries, more bandwidth or more compression may barely help. Tune the common case, then give large objects a separate path.

Implementation: [GCS writer configuration](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/pkg/objectstorage/gcs.go). Next: [parallelizing the whole pipeline](09-parallelize-the-whole-upload-pipeline.md).

[Back to the series](README.md)
