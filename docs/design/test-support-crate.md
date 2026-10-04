# Real Docker test fixtures

Default `cargo test --workspace --all-targets` does not select Docker integration
tests. Select them explicitly with the `integration-tests` Cargo feature:

```sh
export DOCKER_HOST=ssh://user@workstation
cargo test --locked --workspace --all-targets --features integration-tests -- --nocapture
```

`DOCKER_HOST` is an explicit URI interpreted by Bollard, for example
`unix:///var/run/docker.sock`, `tcp://host:2375`, or `ssh://user@host`.
No default socket or Docker context is selected. SSH requires local OpenSSH
and remote `docker` on the non-interactive PATH; SSH URIs cannot select a remote
socket path. TLS certificate discovery follows Bollard.

`test-image.ref` pins the published fixture by digest. `REPRISE_TEST_IMAGE_REF`
can explicitly override that pin; an empty or malformed override fails startup.

When selected, tests fail on missing configuration, connection errors, registry
errors, or unavailable `runsc`. They do not silently skip or substitute a mock.
The only ignored test is the unfinished daemon sandbox suspend/resume scenario.

## Fixture interface

```rust
let fixture = reprise_test_support::TestFixture::from_env().await?;
// Use fixture.docker and fixture.image.id on the same endpoint.
// The future daemon fixture can also use fixture.host.
```

The fixture owns configuration, connection negotiation, and image preparation.
There is no separate connection-wrapper crate. Connection has a 30-second
deadline; image preparation has its own 120-second deadline.

The image must be pinned by registry digest. Only an inspect 404 triggers a pull;
other Engine failures and streamed pull errors fail preparation. There is no
build, archive import, retry, or image fallback. Docker selects the server's
platform. The local image ID comes from Engine inspect, not the registry digest.
Concurrent consumers may each pull on a cache miss, and an Engine pull may
continue after cancellation. Shared images are never deleted or retagged.

## Real tests and CI

`registry.rs` verifies shared cached preparation and rejection of a nonexistent
registry digest against a real Engine. It can run with any published image pin:

```sh
cargo test -p reprise-test-support --features integration-tests --test registry -- --nocapture
```

`fixture.rs` needs the published Reprise test image on a Linux Engine with
`runsc`. It starts two isolated workloads, checks the memory-state contract,
and cleans up only its own containers, including after workload failure.
It does not establish checkpoint/restore capability.

The Check workflow keeps default Rust tests on Linux and Windows and adds a
Linux Docker integration job. That job installs a pinned, checksum-verified
gVisor release and enables `integration-tests`. The installation includes the
sidecar binaries required by current [gVisor releases](https://gvisor.dev/docs/user_guide/install/).

Both CI and development use the checked-in `test-image.ref`, taken from a
successful image publication. The GHCR package is public, allowing anonymous
pulls from fork PRs. Updating the pin is an explicit fixture upgrade, separate
from business-test changes. Consumers never rebuild or import the image.
