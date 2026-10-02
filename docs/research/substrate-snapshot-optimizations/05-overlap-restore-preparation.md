# Restore was waiting for steps that could run together

A restore needs a saved execution state and somewhere to execute it. That creates two substantial preparation jobs: fetch the snapshot, and prepare runtime assets plus the container filesystem.

Running them one after another is a natural implementation. It is also an unnecessary dependency. Image preparation does not consume the downloaded memory image, and snapshot download does not need the finished root filesystem. Only the final runtime restore needs both.

The [overlapped-transfer change](https://github.com/agent-substrate/substrate/commit/9360791710e4ce41fc8e79402ad79873365fef2d) launches those branches concurrently and joins them before invoking the runtime. For independent preparation times `D` and `P`, the serial path pays approximately `D + P`; the overlapped path pays approximately `max(D, P)`. The shorter branch largely disappears from the critical path without either branch becoming individually faster.

## Move the join to the actual dependency

This is more precise than making every operation concurrent. Asset preparation and bundle assembly still have their own ordering requirements. The change overlaps the two branches whose outputs meet at restore, using shared cancellation so a failed branch can stop its sibling.

The same reasoning appears again in the host-backed rootfs work. Extracting the saved writable upper can overlap bundle preparation, as long as both finish before mounting and launching the restored guest. During checkpoint, once the guest is paused, memory capture and filesystem capture can also proceed together.

The payoff depends on cache state. With a warm image cache, filesystem preparation may be tiny, leaving little to hide. A cold worker can spend much longer fetching assets or unpacking an image, making overlap more valuable. This explains why two machines with similar memory-restore timings can still produce very different end-to-end resumes.

**Takeaway:** Draw the dependency graph before tuning the individual steps. Sequential source code often contains waits that the system's actual dependencies do not require.

Implementation: [restore preparation branches](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/cmd/atelet/main.go#L1173-L1267). Related change: [overlapped rootfs capture and extraction](https://github.com/agent-substrate/substrate/commit/c1339e5f020190e4d124007279b18bf4ffc5af5b).

[Back to the series](README.md)
