# reprise-test-support

Prepare a saved test image before starting the daemon or measuring restore time.
Configure `FixedTemplate.image` with the returned immutable ID and use the same
Docker endpoint. Image preparation is fixture setup, not evidence of checkpoint
capability.

```rust
let endpoint = DockerEndpoint::connect(socket_path, Duration::from_secs(120))
    .await
    .expect("connect to test Docker");

let prepared = endpoint
    .ensure_image(archive_path)
    .await
    .expect("prepare test image");
let image_id = prepared.id.as_str();

// Configure FixedTemplate.image with image_id on the same endpoint.
```

`socket_path` is an explicit Unix path string, not a `unix://` URI. Docker
contexts and `DOCKER_HOST` are not consulted. Connection and preparation return
`PrepareError`; tests use `expect` or `?` at the call site. A single
`tokio::time::timeout` bounds `connect` and each `ensure_image`, including
archive reading and Docker calls. That deadline stops the caller's wait only.
A blocking archive read or an engine-side load may continue after
`PrepareTimeout`.

## Archive and sharing contract

Supply an uncompressed Docker save tar containing exactly one image suitable
for the test host. Zero or several images is `ArchiveError::NotSingleImage`.
Multiple tags for that one image are supported. The config is read without
extraction and its original bytes determine the image ID. The helper does not
check platform compatibility or layer integrity. Docker reports load failures
during upload.

The opened file is retained for upload. Test fixtures must stay immutable during
preparation; concurrent-write detection and file locking are intentionally omitted.
Blocking archive reading runs outside the async executor.

Parallel callers can share the endpoint, archive and image. Every call owns
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
  cargo test -p reprise-test-support --test image_preparation -- --nocapture
```

When either variable is unset, the test returns without contacting Docker. With
both set, a workspace `cargo test` runs the upload.
