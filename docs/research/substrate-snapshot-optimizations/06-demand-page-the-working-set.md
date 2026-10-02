# Restore the working set, until the hypervisor stops cooperating

A restored guest may touch only a small part of its saved memory before doing useful work. Loading every populated page immediately raises its idle footprint and spends resources before the workload needs them.

Substrate used Cloud Hypervisor's on-demand restore to fault pages in as the guest touched them. The [restore-mode commit](https://github.com/agent-substrate/substrate/commit/327386942f231676e736f05354294a58e693cd12) reports a telling comparison on kind, arm64, with a 2 GiB counter-demo guest: v52 on-demand restore held about **16 MiB in the guest memfd**, while eager restore on v53 held **158 MiB**. That difference matters when many actors are mostly idle.

Then a hypervisor upgrade changed the workload underneath the optimization.

## An optimization is a dependency on behavior

Cloud Hypervisor v53 background-prefaulted all registered pages during on-demand restore and rejected snapshots while that work ran. In Substrate's reported test, the prefault activity starved the guest so badly that its readiness probe never passed. The actor never became ready.

The response was to choose the memory mode from the actual VMM version, already available in the normal ping response. Versions with the problematic behavior use eager `Copy`; unaffected versions retain `OnDemand`. Unknown versions also take eager restore, favoring a larger footprint over a guest that cannot start. The affected version range has an explicit upper-bound hook for retiring the workaround.

Choosing eager also removes downstream work. Its next checkpoint is self-contained, so the runtime skips the base/delta merge and deletes the staged memory image. In the commit's v53 measurements, those follow-ups reduced median pause time from roughly **0.69 to 0.32 seconds** and per-actor disk usage from **318 to 158 MiB**.

On-demand restore has a different lifecycle: the source image must remain available while the guest can fault from it, and subsequent checkpoints must retain untouched pages from that base.

**Takeaway:** Optimize for the active working set, but make the runtime's real behavior part of the decision. A dependency upgrade can turn deferred work into aggressive background work and erase the assumption that made the optimization useful.

Implementation: [mode selection and source lifetime](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/cmd/ateom-microvm/restore.go), [version gate](https://github.com/agent-substrate/substrate/blob/7317e083cf7a80ed26419eb14ba4f289a6846525/cmd/ateom-microvm/internal/ch/prefault.go).

[Back to the series](README.md)
