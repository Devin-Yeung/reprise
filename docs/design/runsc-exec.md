---
status: draft
date: 2026-10-05
---

# Execute short commands in a running instance

An instance has one long-lived initial process. Reprise must also run short,
caller-requested commands in that already-running instance without rebuilding
its bundle or restarting that process. `runsc exec` is the runtime operation for
that distinction.

`reprise-runsc` exposes this as `Runsc::exec(ContainerId, ExecOptions)`. The
request requires a program path and accepts positional arguments. It captures
stdout, stderr, and the resulting exit status in `ExecutionOutput`; those bytes
remain an application protocol owned by the caller.

The wrapper intentionally does not model a generic OCI process specification
yet. Environment changes, user selection, terminals, signal delivery, file
descriptor passing, cancellation, and deadlines stay outside this first
interface. The direct argv form is enough to invoke `memory-state get` and
`memory-state mutate` in the same instance as its long-lived `serve` process.

`exec` is valid only while an instance is running. Lifecycle orchestration must
not begin a checkpoint while an execution is active. That serialization belongs
to the runtime/control-plane layer, not this blocking CLI adapter.

TODO: add a privileged Linux integration test that starts `memory-state serve`,
uses exec to observe and mutate it, checkpoints it, restores it, then uses exec
again to apply the nonce-and-revision correctness gate.
