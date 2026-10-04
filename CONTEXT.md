# Reprise

Reprise suspends sandboxes into snapshots and brings them back, as a study in making that round trip fast.

## Language

### Sandboxes

**Sandbox**:
An isolated execution environment with a stable identity. That identity persists while suspended and across replacements of its running instance.
_Avoid_: Actor, container

**Instance**:
One running incarnation of a sandbox. A sandbox has at most one; suspend ends it, and every resume starts a new one.
_Avoid_: Container, VM, runtime

**Image**:
The OCI image a sandbox's filesystem is built from, and the starting point of a cold boot.
_Avoid_: 镜像, checkpoint image

**Snapshot**:
A saved state of a sandbox's processes and filesystem from which its execution can continue. It does not include the state of external services or other independently managed resources.
_Avoid_: Checkpoint image, memory dump, 镜像

**Execution**:
One command run inside a running sandbox, together with its output and exit status.
_Avoid_: Operation, job

### Lifecycle

**Suspend**:
Capturing a snapshot of a running sandbox and ending its instance.
_Avoid_: Pause, freeze

**Resume**:
Bringing a suspended sandbox back to running: by restore when it has a snapshot, by cold boot only when it has none.
_Avoid_: Wake, start

**Cold boot**:
Starting a new instance from the sandbox's image, so the application initializes from scratch.
_Avoid_: Cold start

**Restore**:
Starting a new instance from a snapshot, so execution continues where it stopped instead of reinitializing.
_Avoid_: Warm start, hot start, 热启动

**Swap**:
Suspending one sandbox and resuming another in the capacity it released.
_Avoid_: Eviction, replacement

### Measurement

**Time to first response**:
The time from asking for a new instance, by cold boot or restore, until its workload first answers a request.
_Avoid_: Startup time, boot time, resume latency
