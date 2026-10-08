---
status: draft
date: 2026-10-05
---

# Query snapshot state through in-sandbox executions

`memory-state` is the smallest workload that can distinguish restoring an
execution snapshot from starting a fresh process. Its server generates a random
boot nonce once and keeps it, a value, and a revision solely in process memory.

The OCI initial process is `memory-state serve --socket PATH`. It stays in the
foreground, so its lifetime is the instance lifetime. Later executions run
`memory-state get --socket PATH` or `memory-state mutate VALUE --socket PATH`.
Those commands connect to the long-lived process through a Unix-domain socket,
print its JSON state, and exit.

This deliberately avoids a host-facing HTTP listener, veth pair, port allocation,
and IP-address restoration. The socket is local to the sandbox: it is a protocol
between an initial process and a later `runsc exec` process, not a control-plane
endpoint. It requires an eventual runtime `exec` capability, specified separately.

The first snapshot correctness gate is:

1. Start `serve`; `get` records nonce `A` and revision `0`.
2. `mutate` produces nonce `A` and revision `1`.
3. Checkpoint, then restore without rerunning `serve` or replaying the mutation.
4. `get` must produce nonce `A` and revision `1`; another `mutate` must produce
   revision `2`.

Matching the nonce demonstrates that a fresh server did not initialize. The
revision pins the checkpoint point and demonstrates that the restored process can
continue. Filesystem-write persistence, external networking, and host-visible
readiness are separate experiments and are intentionally not part of this gate.

TODO: run this gate against the pinned Linux `runsc` release and record whether a
listening Unix-domain socket survives checkpoint/restore unchanged.
