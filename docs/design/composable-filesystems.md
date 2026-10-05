---
status: draft
date: 2026-10-05
---

# Model an OCI bundle's filesystem as a rootfs with stacked layers

## Problem

`reprise-oci` currently exposes a Nix-specific root and its read-only store
mounts separately. Callers then assemble standard runtime filesystems such as
`/run` and `/proc` in their own bundle code. That makes a workload-specific
writable directory look like test support, repeats OCI mount construction, and
prevents another source of mounts from using the same assembly rules.

## Decision

A `Rootfs` is one read-only root directory plus an ordered stack of `Layer`s.
"Layer" here means a group of mounts applied over the root directory, not an OCI
image layer. Each layer expands to one or more OCI mounts and knows what host-side
setup it needs. Nothing is copied and nothing is mounted by this crate.

`Layer` is a closed enum of the common cases: tmpfs, `/proc`, `/sys` and sets of
bind mounts. A layer is more than a `Vec<Mount>` because it has intent and
behavior: it expands to several mounts that must stay adjacent, it declares
host-side preparation (a bind source must exist, a file bind needs a file as its
target), and it can later contribute spec fields beyond mounts. The Nix store is
one such layer: a closure is a set of read-only binds at their own paths, so no
Nix-specific rootfs type is needed.

`Rootfs` owns the composition rules, so individual layers do not repeat them:

- Layers apply bottom to top, and expanded mounts keep that order because OCI
  mount order is observable.
- Two mounts with the same destination are rejected.
- A mount whose destination is an ancestor of an earlier mount's destination is
  rejected, because it would hide that mount. Nested destinations in
  parent-first order are legal.
- Every destination is an absolute path contained by the root.
- The root path is absolute and the root is always read-only.

Invariants are enforced as layers are added, so a `Rootfs` is always valid and
ready to insert into a bundle spec. Preparation is an explicit operation: it
creates mount targets on the host before the root is made read-only inside the
container.

## Constraints

- A composition has exactly one root directory by construction. Combining two
  roots requires a future explicit overlay adapter, never an implicit merge.
- Layers are independent of process, namespace, networking, runsc invocation,
  and the `memory-state` workload.
- First scope is mounts only. `/proc` and `/dev` in practice also interact with
  `linux.maskedPaths`, `readonlyPaths` and `devices`; adding those means
  extending `Layer`, not changing the composition rules.
- There is no `/dev` layer yet. An empty `/dev` tmpfs is not a usable device
  profile, and which devices gVisor provides on its own is unverified.

## Rejected alternatives

Keeping this in `tests/support` would make production bundle assembly duplicate
the same policy. Putting process and runsc configuration here would make a
filesystem module depend on one runtime.

A layer type that only wrapped `Vec<Mount>` was rejected: without behavior it
adds vocabulary and no guarantees. A separate validated plan type plus builder
was rejected for the same reason: validating on insertion gives the same
guarantee with one type.

A `Layer` trait is a speculative extension seam. Every layer needed today is a
known case; introduce a trait when a layer must be defined outside this crate.

## Open questions

- A production bundle builder will need to decide who owns and removes a
  materialized rootfs directory after `Rootfs::prepare`.
- A `/dev` layer must specify which devices it creates, and whether runsc
  already supplies the defaults. This needs a Linux runsc experiment.
- Whether layers contribute spec fields other than mounts, and through which
  interface.
