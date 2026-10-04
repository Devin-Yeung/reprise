---
status: retired
date: 2026-10-04
superseded-by: snapshot-runtime.md
---

# Reprise test image

`reprise-test-image` is a reproducible Docker image for test workloads, not a
deployment image for the sandbox daemon. It holds the `memory-state` Go workload,
BusyBox, and `tini` as PID 1 to reap detached test processes. No `reprised`,
compiler, interpreter, startup package installation, or host source mount is
involved, so the image never depends on the Rust workspace.

Code: `nix/reprise-test-*.nix`, `test-tools/`,
`.github/workflows/publish-test-image.yml`, and
`crates/reprise-test-support/src/memory_state.rs` for the workload contract.

## Why Nix, publish once, consume by digest

Nix is the only builder: `flake.lock`, `go.sum`, and `vendorHash` pin every
input, and the image has no upstream base image or wall-clock timestamp. No
Dockerfile, archive upload, or local build fallback exists.

Consumers never build. Tests pin the published multi-architecture index in
`crates/reprise-test-support/test-image.ref`; updating it from a successful
publication is an explicit fixture upgrade, independent of ordinary Rust
changes. No `latest` tag is used. Docker selects the image for the **server's**
architecture, so a Mac driving an amd64 Engine consumes the amd64 image.

The publisher verifies only the platform, never runs `memory-state`: the
workload contract belongs to the integration tests and may change without a
publisher update. Publication is not a checkpoint/restore test; those also need
a Linux host with a supported `runsc` configuration.

The GHCR package must be public for anonymous pulls from developers and fork
PRs; repository visibility alone does not make a new package public.

## Why the workload looks like this

`memory-state` keeps a random boot nonce, value, and revision only in process
RAM, with no state file, seed, or replay log. An unchanged nonce after resume is
therefore evidence the same process survived, not that it was restarted and
rehydrated. `start` detaches fully and reports readiness only after binding, so
no Execution stdio or pin keeps the launcher alive, and it refuses to adopt an
existing listener's state.

## Extending

Add Go commands under `test-tools/cmd/` and list them in
`nix/reprise-test-tools.nix`; add packages to `nix/reprise-test-runtime.nix`.
Further images are separate Nix expressions; there is no base-image hierarchy.
