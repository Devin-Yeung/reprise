---
status: draft
date: 2026-10-04
---

# First integration test: in-memory HTTP state continuity

The acceptance scenario compiles; the concrete daemon fixture is not
implemented. Explicit execution prepares the fixture image, then fails before
creating a sandbox.

## Run

```sh
DOCKER_HOST=ssh://user@workstation \
  cargo test -p reprise-daemon --test checkpoint_restore -- --ignored --nocapture
```

Setup uses `reprise_test_support::TestFixture`, so `DOCKER_HOST` and the
optional `REPRISE_TEST_IMAGE_REF` override behave as in
[the test-support fixtures](test-support-crate.md): the digest-pinned [Reprise
test image](test-image.md) is pulled if absent and its endpoint-local image ID
is passed to the daemon.

This is the only ignored test: concrete daemon startup and sandbox suspend/resume
are not implemented. Its ignore marker tracks that missing implementation,
not infrastructure availability. All implemented Docker integration tests use
the `integration-tests` feature and fail when configured infrastructure is unavailable.

## Seam and workload

The fixture will start the real daemon using the configured Docker host, a
temporary local SQLite database and snapshot directory, and the prepared
`reprise-test-image`. The test image contains only test tools, not `reprised`.
These host-side temporary directories are not container mounts. Remote
checkpoint artifact transfer remains an implementation question; a forwarded
Docker socket alone does not solve it.

The scenario in `crates/reprise-daemon/tests/support/mod.rs` takes a real
`SandboxService` and resolved `TemplateId`. It never calls Docker directly.

1. Create a sandbox with auto-suspend disabled.
2. Execute `/bin/memory-state start` to spawn a detached Go HTTP server. Detach
   all stdio and wait for readiness before the launcher exits.
3. Read the startup nonce, generated randomly inside the server and held in RAM.
4. Generate a fresh test-side random value after startup and mutate the server.
5. Suspend without force, wait for success, and inspect the committed snapshot
   and suspended state. Do not execute while suspended: that would wake it.
6. Resume explicitly and require the operation to identify the committed
   snapshot, rather than a cold boot.
7. Read the server state without replaying startup or mutation; require the same
   nonce, value, and revision.
8. Apply a new mutation and require revision 2, proving the server is live.
9. Destroy only this test's sandbox, including after scenario errors/timeouts.

HTTP requests use `memory-state state` and `memory-state mutate VALUE` through
`SandboxService::execute`, with output collected via `execution_events`.
No inline source, interpreter, published ports, runtime package installation,
mounted source files, or workspace writes are required. Image publication and
pulling are separate from the scenario.

## What remains red

The entry test explicitly fails at the daemon-fixture wiring point. There is no
concrete daemon or `SandboxService` implementation to start, and the public
`TemplateId` currently has no caller-accessible construction/parsing interface.
Once those exist, wire startup to `support::verify`, then shut down the daemon.
Do not introduce a fake service or direct-Docker fallback to make this test pass.

A successful run would prove process-memory continuity through the public
interface. It would not independently prove snapshot artifact portability,
workspace rollback, or that an implementation serialized instead of merely
freezing an instance. Those require separate runtime/artifact validation.

The initial runtime target may retain the same stopped Docker container for
restore; the scenario does not require deleting and recreating it. Unsupported
checkpoint capabilities must fail, not silently skip or substitute a cold boot.
