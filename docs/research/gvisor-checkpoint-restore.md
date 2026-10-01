# Docker + gVisor: saving and restoring process state

Status: primary-source review, not a validated runtime experiment. No Docker/runsc checkpoint or restore has been performed in this project, and no performance conclusions have been established.

## Conclusion

Yes. gVisor's native checkpoint/restore saves running process state and resumes execution rather than rerunning the application's startup command. The official documentation describes both direct `runsc` and Docker workflows.[1]

The more accurate mental model is "serialize and reconstruct the sandbox's execution state," not "replace a running container's RAM in place." gVisor's Sentry is a userspace application kernel that manages processes, memory, filesystems, and network objects inside the sandbox. Restoring execution requires both application memory and the corresponding kernel state.[1][2]

## Docker workflow and boundaries

The official command outline is below. It assumes a Linux Docker Engine correctly configured with `runsc` and a compatible Docker/containerd/runsc combination. This is neither an installation script nor a locally validated result.[1][3]

```sh
docker run [options] --runtime=runsc --name=actor <image>
docker checkpoint create actor checkpoint-001
docker start --checkpoint checkpoint-001 actor
```

- Docker checkpoint is still marked experimental.[3]
- The Docker workflow documented by gVisor restores into the same Docker container; the direct `runsc` workflow demonstrates creating a new container before restoring. Direct `runsc` capabilities must not be treated as proof of Docker Engine support for restoring into new containers, cloning, or migration.[1]
- Reusing the same Docker container does not mean the same host runtime process remains alive throughout. Application execution continuity and host instance identity are distinct concepts. The gVisor documentation notes that even `--leave-running` performs an immediate restore, which may change the host process ID.[1]
- Docker's general checkpoint documentation describes CRIU; gVisor has its own state-saving and restoration mechanism. These must not be conflated.[1][2][3]

## A snapshot does not capture the entire external world

- A snapshot may consist of multiple files rather than a single memory dump. Uncompressed execution snapshots allow kernel state and memory to be restored in parallel.[1]
- Filesystem coverage must be explicit. gVisor provides a separate `fscheckpoint` mechanism that saves rootfs upper layers by default. Mount coverage depends on configuration and supported conditions. These filesystem snapshots can only be restored by the same `runsc` binary that produced them.[4]
- A successful execution checkpoint does not prove that all host bind mounts or external volumes have consistent, rollback-ready copies. This is a design inference from the separate filesystem snapshot mechanism and its coverage limits; each case requires experimentation.[4]
- Network continuity is not unconditional. With `--network=host`, connected sockets report connection resets after restore, and applications must reconnect. External service state and already-completed API side effects are outside the local sandbox's restoration scope.[1]
- The restoring host CPU must support the CPU features enabled in the snapshot. Cross-machine restoration cannot be promised unconditionally.[1]

## Implications for Reprise

Currently, `crates/reprise/src/lib.rs` only re-exports the API and contains no Docker/runsc implementation. `crates/reprise-api/src/capabilities.rs` models process checkpoint/restore, same-container restoration, and workspace snapshots separately. Its contract requires runtime probes to establish capabilities and forbids silently substituting a cold boot when restoration is unavailable. This is an API contract, not a verified runtime capability.

Suggested minimal validation: keep a random marker and an incrementing counter exclusively in a long-running process's memory. After checkpointing, confirm that execution has stopped. Restore and verify that the marker is unchanged and the counter continues from its previous value rather than being initialized again. Then separately validate process trees, open files, file modifications, and external connection re-establishment. This is a validation proposal, not an experimental result.

## Primary sources

1. [gVisor — Checkpoint/Restore](https://gvisor.dev/docs/user_guide/checkpoint_restore/)
2. [gVisor — Introduction to gVisor security](https://gvisor.dev/docs/architecture_guide/intro/)
3. [Docker — docker checkpoint](https://docs.docker.com/reference/cli/docker/checkpoint/)
4. [gVisor — Filesystem Snapshots](https://gvisor.dev/docs/user_guide/fs_snapshot/)
