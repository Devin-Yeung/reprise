# Direct runsc vs Docker + runsc for Reprise v1

Status: primary-source research and recommendation, not an accepted architecture decision or a validated runtime experiment. No container, checkpoint, or restore was executed for this research.

## Conclusion

**runsc can run an OCI container directly on a Linux host; Docker is not required.** The official OCI quick start explicitly demonstrates direct runtime invocation. Its use of Docker to export a sample rootfs is preparation, not a dependency of the running sandbox.[1][2]

**Updated deployment constraint:** the developer runs Reprise locally on macOS and wants both development and CI to use a remote Linux execution host through a forwarded Docker socket, without deploying each newly built Reprise binary there. Under this constraint, prefer **Docker + runsc for v1**. Direct runsc does not provide an equivalent supported remote Engine API; see section 7. The direct-runsc recommendation below is conditional on co-locating the runtime-owning daemon with runsc, which does not match the requested development workflow.

For a co-located Linux deployment and Reprise's narrow first acceptance scenario, a **direct-runsc experiment, then a direct-runsc v1 if that experiment passes** remains attractive, provided v1 accepts these constraints:

- The runtime-owning daemon runs on the Linux execution host, with local access to bundles and checkpoint directories.
- A fixed, prebuilt rootfs/template is provisioned outside sandbox creation; no arbitrary registry images yet.
- One sandbox per runtime container; no external networking initially (`--network=none`, sandbox loopback still available).
- Use a controlled privileged deployment for the initial experiment; do not assume `runsc --rootless` supports restore.
- Pin the runtime distribution and configuration, and fail closed if the real memory-continuity probe fails.

This recommendation is an engineering inference based on the scope and the documented interfaces, not a performance claim. If arbitrary images, bridge networking, or a remote Docker-socket-only deployment are v1 requirements, Docker becomes substantially more attractive.

## 1. What each layer does

An OCI **runtime bundle** consists of `config.json` and a referenced root filesystem. It is not interchangeable with an OCI image layout or a Docker image archive. The runtime consumes prepared filesystem/configuration, rather than serving as a registry/image manager.[1][2]

The official direct workflow is:

```sh
# In a prepared bundle directory containing rootfs/:
runsc spec -- /hello
sudo runsc run hello
```

The documentation prepares the example rootfs using `docker export`; a rootfs produced by another build/provisioning mechanism satisfies the same bundle boundary. Reprise could produce it from its existing Nix runtime closure, but that artifact has not been implemented or tested.[1][2]

Docker Engine is a higher-level daemon with APIs and a CLI. It manages images, containers, networks, and volumes.[3] It is therefore more than an unnecessary forwarding layer, but it is also not the Reprise scheduler: Reprise still owns logical sandbox identity, suspend/resume policy, serialization, durable lifecycle records, and snapshot commit ordering.

Conceptually:

```text
Docker route: Reprise -> Docker Engine API -> runtime integration -> runsc
Direct route: Reprise -> runsc CLI -> gVisor sandbox
```

These diagrams describe responsibility boundaries, not a complete process tree.

Direct invocation still launches gVisor's runtime processes; it is not simply chroot and does not mean Reprise must implement a userspace kernel. For ordinary filesystem mounts, gVisor uses a Gofer process in addition to its Sentry.[4]

## 2. Checkpoint/restore is native to runsc

The official guide documents raw commands:[5]

```sh
runsc run <container-id>
runsc checkpoint --image-path=<unique-directory> <container-id>
# Prepare a new runtime container from the matching bundle/configuration:
runsc create <restore-container-id>
runsc restore --image-path=<unique-directory> <restore-container-id>
```

This is a command outline, not a ready-to-run Reprise fixture. Bundle paths, global flags, privilege, detached process hosting and cleanup must be specified for the chosen release. Do not reuse a checkpoint directory for another checkpoint.[5]

- Without `--leave-running`, checkpoint stops the container processes by default.[5]
- Direct runsc supports restoring execution state into a new runtime container. The Docker workflow in the same guide restores into the original Docker container and documents new-container restoration as unsupported.[5]
- `--image-path` gives the caller an explicit artifact directory, avoiding dependence on Docker-managed checkpoint paths. This helps only if the caller can access that host filesystem; remote runsc still requires remote execution and artifact handling.[5]
- Current restore source can create a missing container from a bundle, and otherwise loads an existing runtime record. Use the documented sequence until a pinned-release experiment validates any simplified sequence.[6]
- The CLI labels checkpoint and restore experimental. Docker's checkpoint interface is also experimental; adding Docker does not make the underlying capability proven.[6][7][8]

There is already direct exec support (`runsc exec`, including a process JSON specification), so Docker is not required to run the existing `memory-state` commands. Reprise still needs to supervise exec, collect stdout/stderr and exit status, and prevent checkpoint during an active execution.[9]

## 3. What we gain and what we take over

The following table combines documented layer responsibilities with Reprise-specific design inferences.[1–9]

| Concern | Docker + runsc | Direct runsc |
| --- | --- | --- |
| Image/rootfs preparation | Engine manages images and container filesystem setup | Provision a rootfs and OCI config; registry pulls, layer unpacking/whiteouts and caching become additional work if required |
| External networking | Engine manages networks | Reprise/provisioning must arrange namespaces, interfaces/routes and optionally CNI; not needed for the first loopback-only test |
| Exec/output | Engine API is available | Native exec exists; Reprise supervises CLI processes, pipes, signals and exit status |
| Runtime lifecycle | Engine container records/API | Reprise manages runtime IDs, state queries, waits, termination and orphan cleanup |
| Snapshot artifacts | Runtime functionality exposed through Docker's checkpoint API and host paths | Native checkpoint/restore with an explicit caller-selected directory |
| New-container restore | Official gVisor guide documents a same-container restriction | Native direct workflow supports a new runtime container |
| Resource/security policy | Engine offers a higher-level configuration surface | Reprise must generate/validate OCI policy and configure supported cgroup/resource controls; runsc still performs runtime mechanics |
| Remote development | Existing Unix-socket tunnel can reach the Engine API | A socket tunnel is not a runsc interface; run daemon/tests on Linux or introduce a remote execution mechanism |
| Operational dependencies | Engine plus its runtime integration and runsc | No Engine, but Linux, runsc distribution, privileges, bundles and host supervision remain |

**Removing Docker reduces dependencies, not necessarily implementation work.** It is attractive here because a fixed rootfs and loopback-only workload let us deliberately leave most Engine features out of v1, rather than reimplement them.

No latency, memory overhead or throughput advantage is established by this review. Both routes execute the workload under gVisor; any comparison needs measurements.

## 4. Important boundaries

### Linux and privileges

The current installation documentation requires Linux and supports x86_64/ARM64. A macOS developer still needs a Linux host or VM; Docker removal does not remove that requirement. Follow the pinned release's requirements rather than assuming the current docs describe an older installed binary.[10]

The built-in `runsc --rootless` mode explicitly does not support save/restore; the inspected restore command rejects that configuration. Other user-namespace/rootless arrangements are distinct and must be tested separately. Do not conflate direct runsc with an unprivileged deployment.[6][11]

Current installation documentation also describes a runtime distribution with companion `gvisor-bin/` binaries. Pin and provision the complete distribution, not just an assumed standalone executable.[10]

### Networking

`--network=none` retains a sandbox-local loopback, so the existing HTTP memory canary can run without a bridge, published ports, or external connectivity.[12] Avoid host networking merely to make setup easier: it weakens network isolation, and existing connected host sockets do not transparently survive restore.[5][12]

### Filesystem and memory are separate contracts

Restoring application execution does not prove arbitrary bind mounts or workspace data were rolled back. gVisor documents filesystem snapshots separately, with overlay/mount coverage requirements and same-binary restoration restrictions.[13]

For the first memory test, retain the required rootfs/mount assets on the same host and restore under matching configuration. A later portability test must explicitly account for those assets. A new runtime container is not proof of cross-host portability or cloning safety.[2][5][13]

## 5. Recommended experiment and implementation order

1. Provision a pinned runsc distribution on the target Linux host.
2. Build/provision a fixed rootfs containing the existing Nix runtime closure (`memory-state`, BusyBox, tini), plus an explicit OCI config. Prefer a build-time rootfs export over writing a generic image importer.
3. Start one sandbox with external networking disabled; start the memory server via native exec.
4. Record its random boot nonce; mutate a fresh test-side value to revision 1.
5. Checkpoint into an explicit candidate directory; verify runtime processes stopped and artifacts exist.
6. Restore from that checkpoint into a new runtime container with the matching bundle/configuration and filesystem assets retained.
7. Without restarting the server or replaying mutations, require the same nonce, value and revision; mutate again and require revision 2.
8. Clean up only the experiment's containers and directories, including failure paths. Record exact runtime version, config, commands, exit statuses and artifact layout.

This is an independent runtime experiment, not a direct-runtime fallback inside the existing `SandboxService` acceptance scenario.

If it passes, implement one direct-runsc runtime adapter and keep the existing public service seam. Preserve logical `SandboxId` independently of runtime container IDs, per-sandbox serialization, SQLite lifecycle records and candidate -> verify -> commit snapshot ordering. No second backend or generic image engine is needed yet.

If it fails, investigate the specific pinned-runtime/configuration problem. Do not infer that Docker would fix it without testing Docker on that same host.

## 6. Impact on the current repository

Current configuration and fixture contracts explicitly require a Docker socket and preloaded image. Choosing direct runsc is a real architecture/configuration change, not just replacing a subprocess command:

- `crates/reprise-daemon/src/config.rs`: runtime distribution/path, runtime-state directory and prepared template/bundle replace the Docker connection and image contract.
- `crates/reprise-daemon/tests/checkpoint_restore.rs`: fixture runs on Linux with the direct runtime and prepared rootfs; the acceptance scenario still calls `SandboxService` only.
- `nix/reprise-test-runtime.nix`: reuse the closure, adding an explicit rootfs/bundle build artifact.
- `docs/design/first-integration-test.md`, `test-image.md` and control-plane notes: update endpoint, provisioning, capabilities and artifact assumptions after the decision.

No existing contracts were changed by this research. The recommendation deliberately revises the earlier Docker-first assumption because native checkpoint artifact access and the fixed-workload scope make the direct route worth validating first.

## 7. Does runsc have a remotely forwardable management socket?

**Not an equivalent to the Docker Engine API in the documented direct-runsc interface.** The source does contain a per-sandbox Unix-domain control socket, but that is a different abstraction.[14][15]

- The socket is created for a sandbox and addressed using sandbox/runtime state. It is not a host-wide Engine endpoint that accepts image preparation and creation of arbitrary new sandboxes.[14]
- CLI-side code still performs host-local work: bundle/state access, process creation, filesystem operations, and opening checkpoint files.[6][7][14]
- The control protocol is uRPC. Its `FilePayload` transfers open file descriptors using `SCM_RIGHTS`, including stdio, Gofer and restore resources. These are host-kernel handles, not file contents serialized for a remote client.[14][15]
- An ordinary SSH Unix-socket forward relays a byte stream; it does not transport local file descriptors into the remote host's kernel. Consequently, forwarding a sandbox control socket is not a substitute for running the runtime-management code on that host. This is an inference from the protocol's FD-transfer requirements, not a tested tunnel experiment.[15]

This does not claim that no internal RPC can ever be called remotely; it means the internal control socket is not a supported drop-in remote create/exec/checkpoint/restore API for Reprise.

Three practical routes:

| Route | Fit for local development against remote Linux |
| --- | --- |
| Docker Engine + runsc | Best fit to the stated constraint: local Reprise uses Engine API through the explicit forwarded socket; no per-build remote Reprise deployment |
| Native runsc through SSH commands | Technically possible without copying Reprise, but Reprise must own remote command execution, stdout/exit status, rootfs provisioning, artifact transfer, permissions and cleanup; this is an SSH-backed node adapter, not a runsc socket mode |
| Persistent remote agent + runsc | Agent exposes a remote API; works, but introduces a remote component/protocol and its deployment/versioning, which v1 wanted to avoid |

Containerd is also a real daemon alternative, with a socket API and a runsc shim documented by gVisor.[16] It is not an API provided by runsc itself; adopting it adds another client/integration choice. Its exact remote exec and checkpoint behavior would need separate validation, so it is not recommended merely to avoid Docker for this first slice.

**Revised v1 recommendation:** keep the current Docker socket configuration, and exercise the same Docker runtime adapter in development and CI. Running native runsc only in CI would validate a different integration stack, not prove the Docker acceptance path.

The remaining remote-artifact problem is independent: a Docker socket reaches Engine operations but does not generally expose arbitrary host checkpoint directories for local snapshot commit. V1 must separately choose a narrowly scoped artifact retrieval mechanism (for example an explicit SSH/SFTP transfer), or explicitly scope snapshots to the remote host and revise the storage contract. The current local verified-artifact commit contract cannot silently become a Docker checkpoint-name record.

## Primary sources

1. [gVisor OCI Quick Start](https://gvisor.dev/docs/user_guide/quick_start/oci/).
2. [OCI Runtime Specification: Filesystem Bundle](https://github.com/opencontainers/runtime-spec/blob/main/bundle.md).
3. [Docker Engine overview](https://docs.docker.com/engine/).
4. [gVisor Filesystem](https://gvisor.dev/docs/user_guide/filesystem/).
5. [gVisor Checkpoint/Restore](https://gvisor.dev/docs/user_guide/checkpoint_restore/).
6. [runsc restore source](https://github.com/google/gvisor/blob/d3d566ad1979a92cb76918a18cba6e51fe252393/runsc/cmd/restore.go).
7. [runsc checkpoint source](https://github.com/google/gvisor/blob/d3d566ad1979a92cb76918a18cba6e51fe252393/runsc/cmd/checkpoint.go).
8. [Docker checkpoint CLI reference](https://docs.docker.com/reference/cli/docker/checkpoint/). Its general CRIU description must not be mistaken for gVisor's native implementation.
9. [runsc exec source](https://github.com/google/gvisor/blob/d3d566ad1979a92cb76918a18cba6e51fe252393/runsc/cmd/exec.go).
10. [gVisor Installation](https://gvisor.dev/docs/user_guide/install/).
11. [gVisor Rootless](https://gvisor.dev/docs/user_guide/rootless/).
12. [gVisor Networking](https://gvisor.dev/docs/user_guide/networking/).
13. [gVisor Filesystem Snapshots](https://gvisor.dev/docs/user_guide/fs_snapshot/).
14. [runsc sandbox implementation: control sockets and host-side operations](https://github.com/google/gvisor/blob/d3d566ad1979a92cb76918a18cba6e51fe252393/runsc/sandbox/sandbox.go).
15. [gVisor uRPC: FilePayload and SCM_RIGHTS](https://github.com/google/gvisor/blob/d3d566ad1979a92cb76918a18cba6e51fe252393/pkg/urpc/urpc.go).
16. [gVisor Containerd Quick Start](https://gvisor.dev/docs/user_guide/containerd/quick_start/).

Sources were retrieved for this review. Source-code links pin the inspected upstream revision; website documentation is live and may not match the target host's installed release. In particular, the live guide's compression default differs from the inspected command's default initialization, reinforcing the need to record explicit flags and validate the actual binary.
