# The sandbox, the bundle, and runsc

Reprise times `runsc`. The interesting milliseconds are in the arguments, the bundle, and the platform. They disappear if Docker, containerd, and the shim sit between the benchmark and `runsc`.

## What is running

gVisor is an application kernel. It runs in userspace and handles the sandboxed program's system calls itself, in a component called the **Sentry**, written in Go. The program's processes are Sentry processes. `top` on the host does not list them. When the Sentry needs something from the host that the sandbox was allowed to have, it asks a companion process, the **Gofer**, which is the one allowed to touch host files.[1]

The host process tree for one sandbox is small: the Sentry, the Gofer, and the `runsc` commands that started them. The application's own processes, threads, and open files live inside the Sentry's checkpoint, which is why restore can bring them back without rerunning `main`.

`runsc` is the command-line OCI runtime, in the role `runc` plays for an ordinary container. `RuntimeConfig.runsc` is the path to that binary. A snapshot restores only with the same `runsc` version that wrote it.[2]

## The bundle is the input `runsc` accepts

A bundle is a directory containing `config.json` and a root filesystem. `config.json` is required and must have that name. The root filesystem is the directory named by `root.path` in that file.[3] `runsc spec` writes a starting `config.json`. The OCI quick start then runs the bundle with `sudo runsc run`.[4]

`Stage::Bundle` is Reprise writing that directory: the rootfs path, the program's arguments and environment, and the namespaces the spec lists. The arguments `runsc` will receive are built beside that, so they can be unit-tested without executing `runsc`. Cold boot is `runsc create` and `runsc start` (`Stage::RunscCreate`, `Stage::RunscStart`). Restore skips the program's start and loads a snapshot instead (`Stage::RunscRestore`).[2][5]

The bundle is independent of the snapshot bytes. That split is what lets preparation overlap, covered in the memory note.

## Platforms: how the Sentry hears a system call

A **platform** is the mechanism that delivers the application's system calls and page faults to the Sentry. gVisor's architecture guide lists two supported platforms.[1]

**Systrap**, the default, uses `seccomp-bpf` to intercept system calls rather than to filter them down to a workload-specific allow-list. It does not need hardware virtualization, which is why it runs inside a VM. `Platform::Systrap` selects it.[1][2]

**KVM** uses hardware virtualization. The application's code runs in guest ring 3, and `/dev/kvm` has to exist. Nested virtualization (KVM inside a VM such as WSL2) works and is generally slower than systrap in that setup.[1] `Platform::Kvm` selects it. The plan keeps systrap as the baseline and treats KVM as its own experiment, because it moves every number.[5]

A snapshot restores only on the platform it was taken on.[2] Comparing a systrap snapshot restored under KVM is an incompatibility, not a performance result.

During the design discussion, the workstation's Docker registration for `runsc` passed `--platform=ptrace`. The architecture guide's supported platforms are systrap and KVM.[1] A number taken through that Docker registration would be a number for whatever platform that global configuration selects, and changing the platform would mean editing the daemon configuration. The runtime calls `runsc` itself so each invocation carries its own `--platform`.

## Namespaces, briefly

A namespace wraps a global kernel resource so that processes inside it see their own instance. Linux has namespaces for cgroup roots, System V IPC, the network, mount points, process IDs, clocks, user and group IDs, and the hostname.[6]

You will see several of these in `config.json`, because an OCI runtime sets them up. Reprise constructs one of them directly: the network namespace, in the next note. The mount namespace and the user namespace are part of gVisor's own setup. The Sentry runs with a restricted view of the host filesystem and drops privileges before the untrusted program runs.[1] The Gofer, in directfs mode, enters a mount namespace, bind-mounts the container files, and `pivot_root`s into them.[7] That is gVisor's code, not a module you reimplement. Reading the namespace list is enough to recognize the fields in `config.json`.

**cgroup v2** is the kernel interface that accounts for and limits CPU and memory for a set of processes.[8] The workstation uses it. Phase 1 does not set cgroup limits. The word comes up when you read `ps` output or the workstation notes, and when gVisor says the Sentry is placed in cgroups as part of its own confinement.[1]

## Why the benchmark does not go through Docker

`runsc checkpoint` and `runsc restore` are `runsc` commands. Docker exposes them as `docker checkpoint create` and `docker start --checkpoint`, and that path restores into the same container. gVisor's documentation says Docker does not support restoration into a new container.[9] The checkpoint directory, the rootfs, and the `runsc` flags then belong to the daemon. A timed restore would include dockerd, containerd, and the shim, and those stages would not be `Stage::Bundle` or `Stage::RunscRestore` in this process.

The plan builds the bundle and invokes `runsc` as root on the Linux host.[10] gVisor needs that privilege to set up its userspace network stack, then drops privileges before untrusted code runs. Rootless mode is documented for a sandbox with `--network=none`.[1] Reprise creates a network namespace, so `Runtime::new` fails unless it is root.

## Where you meet this

| When you are changing | The concept you need | What it decides |
| --- | --- | --- |
| `Stage::Bundle` and the `runsc` argument builder inside `reprise-runtime` | OCI bundle, `config.json`, rootfs path | The spec the sandbox is created from. Unit-test the JSON and the arguments without `runsc`. |
| `Platform` on `RuntimeConfig` | Systrap, KVM, nested virtualization | Baseline is systrap. A KVM sample is a different experiment. Restore uses the platform recorded in the snapshot. |
| `Runtime::new` | Root, and why | Network setup needs it. macOS runs unit tests only. |
| The decision in [ADR 0001](../adr/0001-drive-runsc-directly.md) | Docker, containerd, the shim | Those processes own the steps the benchmark has to time. Direct `runsc` is how the stages stay visible. |
| `config.json` fields you did not write logic for | Namespaces other than net, cgroup v2 | Recognize them. The network namespace is the one this code creates. |

## Sources

1. [Introduction to gVisor security](https://gvisor.dev/docs/architecture_guide/intro/). Sentry, Gofer, systrap, KVM, nested virtualization, privilege.
2. `RuntimeConfig`, `Platform`, and `Workload` in `crates/reprise-runtime/src/runtime.rs`.
3. [OCI Runtime Spec: Filesystem Bundle](https://github.com/opencontainers/runtime-spec/blob/main/bundle.md).
4. [gVisor OCI Quick Start](https://gvisor.dev/docs/user_guide/quick_start/oci/).
5. [Snapshot runtime](../design/snapshot-runtime.md), "Shape", "Measurement", and "Optimization candidates". `Stage` is `crates/reprise-runtime/src/stage.rs`.
6. [`namespaces(7)`](https://man7.org/linux/man-pages/man7/namespaces.7.html).
7. [Faster filesystem access with Directfs](https://gvisor.dev/blog/2023/06/27/directfs/). Gofer mount namespace. Background; phase 1 does not configure directfs.
8. [Control group v2](https://docs.kernel.org/admin-guide/cgroup-v2.html).
9. [gVisor Checkpoint/Restore](https://gvisor.dev/docs/user_guide/checkpoint_restore/), "How to use checkpoint/restore in Docker" and "Issues Preventing Compatibility with Docker".
10. [ADR 0001: Drive runsc directly](../adr/0001-drive-runsc-directly.md).

Next: [memory, checkpoint, and restore](04-memory-checkpoint-and-restore.md).
