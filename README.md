# **Reprise** — Pause an environment. Resume its state.

Reprise is an early-stage control plane for agent sandboxes, built on gVisor.
We are exploring the edges of two questions:

- **Agent sandboxes at scale.** How do thousands of sandboxes share one host?
- **Pause and resume, fast.** How fast can a gVisor sandbox come back from a snapshot? 

> [!WARNING]
> Early stage. The design is still moving; expect breaking changes.
