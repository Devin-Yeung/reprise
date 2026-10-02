# Provisioning the test image through a Docker socket

Status: verified against the Docker Engine API specification, not tested against the remote endpoint. Proposed fixture change; existing external-preparation contract remains unchanged.

## Supported operations

The Engine API supports both required operations through its ordinary HTTP API over a Unix socket, including an externally forwarded socket:

- `GET /images/{name}/json`: inspect by image name or ID; 200 means found, 404 means absent. Other errors must fail setup, not be treated as a cache miss.
- `POST /images/load`: stream a saved image archive as `application/x-tar`. This is the API behind image loading, not a registry push or a rootfs import.

Sources: [Engine API v1.48 specification, ImageInspect and ImageLoad](https://github.com/moby/moby/blob/v28.0.0/docs/api/v1.48.yaml). Negotiate/use an API version supported by the target Engine; the cited version is an evidence reference, not a proposed forced minimum.

## Identity

Use the Docker image ID (`sha256:...`), not the checksum of the archive. The API defines image ID as the digest of image configuration, which includes layer digests. Registry manifest digests (`RepoDigests`) are different identifiers.

For the single-platform saved image produced by the existing Nix build, fixture preparation can compute the expected image ID from the exact configuration JSON bytes referenced by the archive's `manifest.json`. Validate the archive format and selected entry; do not hash reserialized JSON or assume the config filename is always its hash. Archive hashing remains useful for transport integrity, but cannot be passed to image inspect as an image ID.

A build-produced sidecar with expected image ID and platform is another option, provided it is derived from and checked against the actual archive.

## Proposed fixture flow

1. Build the image archive outside the scenario, for the remote Linux host's architecture.
2. Resolve expected immutable image ID from that artifact.
3. Inspect that ID through the explicit configured socket.
4. Only on 404, stream the archive to image load; consume the complete response stream and check for load errors.
5. Inspect the expected ID again, require identity/platform match, and fail if missing.
6. Pin the template/container create request to that ID, not a mutable tag. Do not silently pull a different image.
7. Run the existing public `SandboxService` acceptance scenario.

This belongs to infrastructure fixture preparation, not the scenario or the production service. It needs no registry or image upload over a separate SSH file-transfer channel. Upload/build/inspect timings must not be counted as snapshot restore latency.

Do not remove the cached image during per-test sandbox cleanup on a shared endpoint. Parallel loads may duplicate work; host/cache locking is optional initially. Post-load verification is required regardless.

## Scope impact

Current `docs/design/first-integration-test.md` and `test-image.md` require external image loading and `REPRISE_TEST_IMAGE`. Adopting this proposal means intentionally changing fixture setup to accept an archive (and optionally verified metadata), while keeping image construction explicit and the acceptance scenario Docker-independent.

Image upload is supported by the Engine API. This does not establish an Engine API for downloading arbitrary host checkpoint files; remote snapshot artifact access remains a separate question.
