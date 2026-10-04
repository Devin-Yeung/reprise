# How the host reaches the sandbox

Time to first response ends when the workload answers an HTTP request. The request has to cross from the host into the sandbox. `runsc exec` would also get a process to run inside, and its own cost would sit inside the number you are trying to measure. The plan uses a network namespace and a veth pair instead.

## A network namespace is a private network stack

A network namespace gives its member processes their own network devices, addresses, routes, and ports.[1][2] A program that binds `10.200.0.2:8765` inside the namespace is not listening on the host's `10.200.0.2`. The host has a different stack. Something has to connect the two, or the host cannot send the probe.

`Stage::Network` is Reprise creating that namespace for the instance.[3]

## A veth pair is a cable with two ends

A **veth** device is a virtual Ethernet device, and the kernel creates veth devices in pairs. A packet transmitted on one end is received on the other. If either end is down, the pair is down. The usual container setup puts one end in the host namespace and moves the other end into the sandbox namespace:[4]

```text
ip link add veth-host type veth peer name veth-sandbox
ip link set veth-sandbox netns <sandbox-namespace>
```

The plan assigns `10.200.0.1` on the host end and `10.200.0.2` on the sandbox end. The workload listens on `10.200.0.2` (or on `0.0.0.0` inside its namespace). The benchmark polls `GET /state` on the sandbox address from the host. `Instance::address` returns that address.[3][5]

Every instance uses the same pair of addresses, so the runtime runs one instance at a time. A second instance would bind the same addresses. The `Runtime` is borrowed for the life of the instance for that reason.[5]

## The addresses are part of the snapshot

The workload, and the Sentry's network state, remember the addresses they had at checkpoint. Restore has to present those same addresses, or the restored listener is bound to an address the host is no longer sending to. The plan fixes the addresses so a restore sees what the checkpoint saw.[3]

gVisor documents checkpoint/restore for `--network=sandbox` (its default, a userspace stack), `--network=none`, and `--network=host`. The extra socket rules in that section are specifically for `--network=host`: a listening TCP socket is recreated, and a socket that was connected at checkpoint time returns `ECONNRESET` after restore.[6] The plan has not recorded which `runsc` network mode the veth pair is attached with. Whether identical addresses are enough for the probe to succeed after restore is an open question the first milestone answers by running it.[3] Read the networking section of the checkpoint document before changing that setup. The failure you are looking for is a restore that returns, and a `GET` that never connects.

## This work is on the clock

`cold_boot` and `restore` both contain `Stage::Network`, and it runs before `runsc`. On W1 the application is tiny, so creating the namespace and the pair is a visible fraction of time to first response. The plan's candidate "network ahead of time" creates the namespace before `restore` is called, and takes that work off the measured path.[3] The `network` span is how you see the size of the win. It is a fixed cost: it does not grow with W2's ballast.

The probe stays a host HTTP request. `runsc exec` starts a process through `runsc`'s control socket. That startup would be inside time to first response and would move when `runsc` changes, independently of restore.[3]

## Where you meet this

| When you are changing | The concept you need | What it decides |
| --- | --- | --- |
| `Stage::Network` | Network namespace, veth pair | One end on the host at `10.200.0.1`, one end in the sandbox at `10.200.0.2`. The probe uses the sandbox address. |
| `Instance::address` and the one-instance limit | Fixed addresses | Restore sees the addresses the checkpoint recorded. A second instance would collide, so the runtime is exclusive. |
| The first correctness run on the workstation | gVisor network mode, listening sockets | Confirm the restored listener accepts `GET /state`. The checkpoint document's socket rules for `--network=host` are the checklist if that mode is the one in use. |
| The "network ahead of time" candidate | `Stage::Network` inside the restore span | Move namespace setup before the timed restore. The win shows up on W1 and stays flat as memory grows. |
| The benchmark's readiness loop | HTTP across the veth, rather than `runsc exec` | The sample ends at the first response. Exec's process startup stays out of the sample. |

## Sources

1. [`namespaces(7)`](https://man7.org/linux/man-pages/man7/namespaces.7.html), the network row.
2. [`network_namespaces(7)`](https://man7.org/linux/man-pages/man7/network_namespaces.7.html).
3. [Snapshot runtime](../design/snapshot-runtime.md), "Rootfs and network", "Measurement", "Optimization candidates", and "Open questions". `Stage::Network` is `crates/reprise-runtime/src/stage.rs`.
4. [`veth(4)`](https://man7.org/linux/man-pages/man4/veth.4.html).
5. `Instance::address` and `Runtime` in `crates/reprise-runtime`.
6. [gVisor Checkpoint/Restore](https://gvisor.dev/docs/user_guide/checkpoint_restore/), "Networking".

The study notes end here. The plan's optimization table is the order to apply them: measure the baseline, then change the stage that dominates it.
