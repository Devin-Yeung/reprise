# reprise-test-support

Prepare a saved test image before starting the daemon or measuring restore time.
Configure `FixedTemplate.image` with the returned immutable ID and use the same
Docker endpoint. Image preparation is fixture setup, not evidence of checkpoint
capability.

```rust
let environment = DockerTestEnvironment::connect(
    socket_path,
    Duration::from_secs(120),
).await;

let prepared = environment.ensure_image(archive_path).await;
let image_id = prepared.image.id.as_deref()
    .expect("preparation returns an image ID");

// Configure FixedTemplate.image with image_id on the same endpoint.
```

The socket is an explicit Unix path; Docker contexts and `DOCKER_HOST` are not
consulted. A single `tokio::time::timeout` bounds each preparation,
including archive reading and Docker calls. As test fixture helpers, connection
and preparation panic with `expect` or assertions on failure, so test callers
do not need to propagate setup errors.

## Archive and sharing contract

Use an uncompressed Docker save tar containing exactly one Linux amd64/arm64
image without a variant. Multiple tags for that image are supported. The config
is read without extraction and its original bytes determine the image ID.
Manifest/config reads are bounded; ambiguous references and duplicate entries
are rejected. Docker validates layers during upload.

The opened file is retained for upload. Test fixtures must stay immutable during
preparation; concurrent-write detection and file locking are intentionally omitted.
Blocking validation runs outside the async executor. Timeout bounds the caller's
wait but cannot stop an already running blocking read or Docker-side load.

Parallel callers can share the environment, archive and image. Every call owns
its descriptor and timeout. Concurrent cache misses may each upload identical
content; loading is idempotent by image ID. Preparation imports archive tags but
never deletes or retags shared images. Tests should use the returned ID and
allocate independent containers, ports and writable state.

## Integration test and CI

`.github/workflows/integration-test.yml` runs on pull requests, pushes to `main`,
and manual dispatch. A Linux runner builds the pinned Nix test image, decompresses
it without loading Docker, installs gVisor's complete apt package and registers
`runsc` with the `systrap` platform. Rust tests access its explicit local Unix
socket; no network Docker listener or socket tunnel is needed on the runner.
The gVisor release channel is intentionally floating and its version is logged.
TODO: pin a known-good gVisor package once the runtime compatibility baseline is established.

The smoke test uploads the image and inspects its immutable ID through the same
Docker socket using a separate client. Start with an endpoint without that image
so the test exercises upload. It does not create or start containers; runtime
execution belongs to the daemon tests. The runsc configuration is available for
those future tests but is not required by image preparation itself.

To run against a prepared Linux endpoint:

```sh
# dockerTools' result-test-image is gzip-compressed.
gzip --decompress --stdout result-test-image > /tmp/reprise-test-image.tar
REPRISE_DOCKER_SOCKET=/var/run/docker.sock \
REPRISE_TEST_IMAGE_ARCHIVE=/tmp/reprise-test-image.tar \
  cargo test -p reprise-test-support --test image_preparation -- --ignored --nocapture
```

Ordinary workspace tests compile but ignore this scenario because it needs
a saved image and an external Docker endpoint. Missing setup fails an explicitly requested
run; it never silently skips or substitutes another runtime.
