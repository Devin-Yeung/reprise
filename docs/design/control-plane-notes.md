---
status: retired
date: 2026-10-04
superseded-by: snapshot-runtime.md
---

# Control plane: upstream model, load-bearing invariants, Reprise simplification

A discussion draft, not an accepted design. Sources inspected in the local
`substrate` checkout at `7317e083`; no runtime experiment performed. The
`docs/learning/` notes in that checkout cover the same ground in Chinese.

## 1. How upstream is actually built

Five binaries (`docs/dev/code-layout.md`, `cmd/`):

| Binary | Role | Talks to |
| --- | --- | --- |
| `ateapi` | Control plane. Owns Actor/Template/Atespace/Tag/Worker lifecycle. | Postgres, atelet, clients |
| `atecontroller` | K8s controller. Reconciles `WorkerPool` CRD → Deployment. | K8s API |
| `atelet` | Node supervisor (DaemonSet). Pulls images, assembles OCI bundles, moves snapshots. | ateapi, ateom, object store |
| `ateom-gvisor` / `ateom-microvm` | In-worker-pod runtime coordinator. `Run`/`Checkpoint`/`Restore`/`Terminate`; embeds `atunnel`. | atelet, runsc/Kata |
| `atenet` | Envoy + `ext_proc` router. Actor-aware ingress, wake-on-request, parking. | ateapi, atunnel |
| `podcertcontroller` | Issues short-lived pod certs for mTLS. | K8s API |

Three state planes, chosen by update frequency and latency budget
(`docs/architecture.md`, glossary):

- **Kubernetes** — low-frequency infrastructure: `WorkerPool` CRD → Deployment →
  warm worker pods.
- **PostgreSQL** — high-frequency lifecycle records: `actors`, `workers`,
  `worker_assignments`, `leases`, templates, tags, atespaces. Versioned proto rows.
- **Object store (GCS/S3)** — immutable snapshots, owned by a UID-scoped prefix.

Caveat: the architecture doc says much is aspirational. The *lifecycle machinery*
is real, implemented and tested (the workflows, store, scheduler, workerservice);
the 1B-actor / 100ms scale claims, autoscaling, peer-to-peer state sharing and
control-plane authz are not.

## 2. The ideas that carry the architecture

1. **Actor / Worker / Assignment are three separate things.**
   Actor is logical identity and lifecycle; Worker is physical capacity and
   network location; Assignment is the current binding plus the reservation it
   holds. A Worker hosts *several* Actors (glossary; `worker_assignments` is a
   per-actor table with a unique `actor_uid`), so the 1:1 diagram in
   `architecture.md` is a simplification, and the code is authoritative.

2. **Warm capacity is provisioned by infrastructure, assigned by the control
   plane.** K8s keeps pods warm; the control plane multiplexes Actors onto them.
   Scheduling latency is taken off the request path but is not zero.

3. **Lifecycle is a re-entrant, persisted state machine.** `RESUMING` /
   `SUSPENDING` / `PAUSING` are committed intents, not in-flight flags. Every
   step derives whether its work is done from the persisted record alone and
   persists before returning (`workflow.go` `stepSpan`/`markSkipped`;
   `workflow_resume.go` `ensureVolumesCreated` → `ensureWorkerAssigned` →
   `ensureVolumesAttached` → `ensureAteletRestored` → `finalizeRunning`). A
   re-entered workflow fast-forwards.

4. **Unique execution right is enforced by the store, not by the scheduler.**
   A per-Actor distributed lease serializes lifecycle ops
   (`lease:actor:<atespace>:<name>`). The assignment table's unique `actor_uid`
   makes double-binding impossible; `BindActorToWorker` does
   `INSERT … ON CONFLICT` and runs the admission callback inside the Worker's
   `SELECT … FOR UPDATE` row lock, so two claims for the last slot cannot both
   be admitted (`store/atepg/worker_assignment.go`, `scheduling/scheduling.go`).
   The scheduler reads a watch-fed cache and is explicitly advisory.

5. **State commit and physical action are not one transaction.** A checkpoint is
   a candidate that is verified and committed before it becomes the Actor's
   latest snapshot; the DB commit and the file upload can each fail
   independently, so the workflow persists intermediate state and has a recovery
   path and a `CRASHED` state (`workflow_suspend.go`, `crash.go`).

6. **Routing is by Actor identity, and the first request wakes the Actor.**
   `ate-target-actor: <atespace>/<actor>` selects the target; `Host` does not.
   The router calls `ResumeActor`, then tunnels to the current worker. Requests
   that hit momentary saturation are *parked* within a bounded budget, and
   concurrent requests for one Actor are de-duplicated onto a single resume
   flight (`docs/request-parking.md`). A resume already committed is never
   canceled when a caller gives up.

7. **Snapshots are immutable and owned by UID.** Object names carry the owner's
   UID, so deletion is "collect my own prefix" and a recreated Actor/Tag cannot
   inherit its predecessor's objects (`internal/resources/snapshot.go`). Scope
   (`Full` vs `Data`) is recorded in the manifest and pins the runtime assets
   needed to restore.

8. **Capacity is self-reported by the node.** `WorkerService.SetWorkerCapacity`
   lets a Worker publish what it can host; the DB row is authoritative for
   admission. A workload that knows it is idle asks the control plane to suspend
   it (`RequestActorSuspend` → `SuspendActor`), so "busy" is workload-owned.

## 3. Deployment complexity vs conceptual seams

Upstream's component count is high, but most of it is *deployment* separation
(HA control plane, many nodes, object storage, mTLS, multi-tenancy). Each
deployment boundary also hides a *conceptual* seam. On one machine the
deployment collapses; the seam should not, because the seam is what makes the
single-machine version testable and keeps the distributed version a swap rather
than a rewrite.

| Upstream seam | Invariant it protects | Single-machine equivalent |
| --- | --- | --- |
| `ateapi` ↔ `atelet` (`ateletpb.AteomHerder`) | physical node actions vs lifecycle policy; idempotency keyed by (actor UID, pod UID) | `NodeExecutor` interface, one local impl |
| `atelet` ↔ `ateom` runtime ops | runtime lifecycle is separate from pod lifecycle | same interface, narrower surface |
| router ↔ `ateapi` (`ResumeActor`) | wake-on-request, identity routing, bounded wait, de-dup | resolver + pin inside `SandboxService`; no Envoy |
| `ateapi` ↔ Postgres | durable intent, versioned CAS, lease, unique binding | `StateStore` interface over SQLite |
| `ateapi` ↔ object store | snapshot portability and ownership | `SnapshotStore` interface over a local dir |
| WorkerPool (K8s) | capacity provisioning | out of scope; a slot budget in config |

The reason this matters: a **seam** is an interface with one implementation; a
**speculative abstraction** is a registry of interchangeable implementations.
The first is required for correctness and testing; the second is what the
simplification should avoid.

## 4. What to simplify, and what to keep

**Drop for now (deployment/complexity):**
Kubernetes, CRDs, `atecontroller`; Envoy/xDS/SDS; `podcertcontroller` and the
mTLS mesh; Postgres; GCS/S3; multiple atespaces, tags, egress policies and
authorization; the multi-runtime registry (`ateom-gvisor` *and*
`ateom-microvm`); Golden snapshots and fork/tag; user-visible `Pause` (fold the
node-local checkpoint into the suspend workflow); arbitrary-port ingress;
parking *as a router process* (keep the semantics, not the process).

**Keep, in-process (correctness and shape):**
the Actor / Worker / Assignment split; the persisted re-entrant operation with
version preconditions and a `CRASHED`/recovery path; the per-Actor lease plus
unique binding plus admission-under-lock; intent-vs-physical-action separation
with a retained candidate snapshot; the snapshot manifest with UID ownership and
a recorded scope; wake-by-identity with a bounded wait and per-sandbox de-dup;
a single-writer process with on-boot reconciliation against Docker; and an
explicit execution pin.

## 5. Resolving the "one process" tension in the previous draft

The previous draft said one `reprised` process hosts the HTTP interface, the
lifecycle coordinator, capacity admission and the node executor, and *also* said
not to build a node-protocol trait until a second deployment exists. Those pull
in opposite directions: without the interface, the responsibilities will not
actually stay distinct, and the later split becomes a rewrite.

Resolution: keep the process combined, keep the seams as interfaces with a
single implementation.

- `StateStore` (SQLite): transactions, versioned compare-and-swap, lease,
  unique binding. SQLite's single-writer is an asset here — `BEGIN IMMEDIATE`
  gives the same "authoritative admission under lock" as the Postgres row lock.
- `NodeExecutor`: `run` / `restore` / `checkpoint` / `terminate`, keyed by
  (sandbox UID, generation). The local Docker + runsc implementation is the
  future `atelet`.
- `SnapshotStore`: commit / validate / load a manifest. A local directory is
  the future object store.
- `SandboxService`: the one deep module clients see. It owns resolution, wake,
  pin, admission and the lifecycle workflows.

Do not add a runtime-backend registry, a second snapshot store, or a network
node protocol until a second implementation or a second machine actually needs
one.

## 6. What Reprise has that upstream does not

Upstream routes *network traffic* to a long-running Actor, and the workload
itself decides when it is idle (`RequestActorSuspend`). Reprise drives
*commands and file operations* from the control plane. Two consequences:

- The control plane is in the execution path, not only the wake path. Admission
  and the running pin become first-class, and `execute` must take the pin for
  the lifetime of the execution so a concurrent suspend cannot checkpoint
  mid-command.
- Acceptance and admission should be separate: accept durably (return an
  operation/execution id), then admit capacity asynchronously and stream output.
  This mirrors the router parking a request: the committed resume is not undone
  when the caller stops waiting.

## 7. Learning-project scope

The sections above describe the target shape, not the floor. V0 proves one thing: **state continuity across suspend/resume under a stable identity**, plus the snapshot commit ordering that makes it safe. Everything else is added only when a test or a second node forces it.

**In scope**

- One process, one writer. One runtime implementation (Docker Engine + runsc), one sandbox per container.
- SQLite for sandbox identity, lifecycle state, latest snapshot reference and executions.
- Per-sandbox serialization (an actor task or mutex per sandbox) in place of a distributed lease.
- A local snapshot directory with candidate → verify → atomic commit.
- The public interface: the domain types and the `SandboxService` trait in
  `crates/reprise-api`. That crate is the source of truth for the surface;
  this document does not restate it.
- Continuity tests (memory canary, process tree + FDs, file mutations) as the acceptance bar.
- A runtime capability probe that fails closed (`UnsupportedRuntime`, never a silent cold boot).

**Out of scope until forced**

- Capacity admission, worker/assignment records, distributed lease, explicit version CAS.
- The re-entrant ensure-step workflow and a full `Failed` recovery path; a simple boot reconcile is enough.
- Object storage, golden snapshots, tags/fork, the `Data` snapshot scope.
- Multi-node, the node-executor protocol, a second runtime backend.
- Parking/flight de-duplication, mTLS, authorization, multi-tenancy, atespaces.

**Keep anyway (cheap now, expensive later)**

- `SandboxId` is not the container id.
- snapshot candidate → verify → commit, retaining the last good copy.
- Lifecycle state and the latest snapshot reference are persisted, not in-memory only.
- Exactly one serialization point per sandbox, even if it is just a mutex.
- The capability probe fails closed.

## Open questions

- Protocol: HTTP+SSE vs gRPC for the public surface.
- Template registration: local config vs an API resource.
- Execution output: cursor/sequence model and truncation policy.
- Idle policy: what counts as "idle" when background processes may run (CPU
  usage vs explicit pin), and whether suspend drains or kills.
- Whether the first slice should prove same-container Docker+runsc restore
  (V0) before any control-plane work.

All recommendations remain draft.
