---
status: retired
date: 2026-10-04
superseded-by: snapshot-runtime.md
---

# Real Docker test fixtures

Docker integration tests are opt-in through the `integration-tests` Cargo
feature; default `cargo test` never touches Docker.

Code: `crates/reprise-test-support` (`TestFixture::from_env`, `ensure_image`),
its `tests/registry.rs` and `tests/fixture.rs`, and the `docker-integration` job
in `.github/workflows/check.yml`.

## Decisions

- **Fail, never skip.** Once selected, missing configuration, unreachable
  Engines, registry errors, or a missing `runsc` fail the test. No mock or silent
  skip stands in. The only ignored test is the unfinished daemon suspend/resume
  scenario, and its marker tracks missing implementation, not infrastructure.
- **Explicit endpoint.** `DOCKER_HOST` is required; no Docker context or default
  socket is guessed, so a test never runs against an unintended Engine.
- **One fixture owns setup.** Configuration, connection, and image preparation
  live in `TestFixture`; there is no separate connection-wrapper crate.
- **Pull only, by digest.** No build, archive import, retry, or alternative
  image. Shared images are never deleted or retagged; each test cleans up only
  its own containers.

## Scope boundaries

`registry.rs` needs only a published image pin and any Engine. `fixture.rs`
needs the [Reprise test image](test-image.md) on a Linux Engine with `runsc`; it
proves workload isolation, not checkpoint/restore capability.

CI installs a pinned, checksum-verified gVisor release with its sidecar binaries
(required by current [gVisor releases](https://gvisor.dev/docs/user_guide/install/)).
CI and development share the checked-in image pin.
