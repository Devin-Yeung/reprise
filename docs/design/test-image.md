# Reprise test image

`reprise-test-image` is a reproducible Docker image for test workloads, not a
deployment image for the sandbox daemon. It contains:

- `memory-state`: a small Go binary using chi for HTTP routing and the standard
  library for requests.
- BusyBox: basic shell and process tools; its `sleep infinity` keeps the sandbox
  running without starting the memory workload automatically.
- `tini`: PID 1, which reaps detached test processes.

No `reprised`, Rust/Go compiler, Python interpreter, package installation at
startup, or host source mount is involved.

## Build and publish

Nix is the only image builder. `flake.lock` pins nixpkgs, including the Go
toolchain and runtime tools; `go.mod` / `go.sum` and `vendorHash` lock the
Go dependencies. The image has no upstream base image, uses a fixed creation
timestamp, and contains no Rust workspace sources.

`.github/workflows/publish-test-image.yml` builds on native amd64 and arm64
Linux runners when image inputs change, or on manual dispatch. Each runner
loads its Nix output and checks that Docker reports `linux/$arch`. It does not
execute `memory-state`; that contract belongs to the integration tests and is
expected to change without a publisher update. This is image validation, not a
checkpoint/restore test.

PRs and dispatches on non-main refs only build and verify. Main publishes the
platform images to `ghcr.io/<owner>/<repository>-test-image`; after both jobs
succeed, a separate job assembles a multi-architecture index from their digests.
The run summary and `test-image-reference` artifact contain the immutable
`image@sha256:…` reference. No `latest` tag is used.

On first publication, set the GHCR package visibility to public if developers
and fork PRs should pull without credentials. Repository visibility alone does
not make a newly created GHCR package public.

## Consume, don't build

Tests should pin the published multi-architecture index digest. Updating that
reference is an explicit fixture upgrade, independent of ordinary Rust changes.
Docker selects the image for the **server's** Linux architecture, not the
developer's architecture. A Mac connected to an amd64 Docker host consumes
the amd64 image.

`reprise-test-support::TestFixture::from_env` reads `DOCKER_HOST`,
connects to Docker, and prepares the checked-in registry pin.
`REPRISE_TEST_IMAGE_REF` can explicitly override that pin. See
[test support](test-support-crate.md) for opt-in preparation and workload tests.
The daemon acceptance fixture still requires an image already present on its
endpoint. To prepare that image manually:

```sh
# Use the real image@sha256:… reference from the publication summary.
image_ref="$REPRISE_TEST_IMAGE_REF"
docker_host=ssh://user@workstation
docker --host "$docker_host" image pull "$image_ref"
docker --host "$docker_host" image inspect --format '{{.Id}}' "$image_ref"
```

Use the image ID reported by Engine inspect with the daemon on that same
endpoint. Legacy image stores report the platform config digest; multi-platform
stores can report the index digest instead. Do not derive the local ID from
the registry reference. No local Nix installation,
remote Nix builder, Dockerfile, archive upload, or build fallback is needed.

`crates/reprise-test-support/test-image.ref` records the published multi-architecture
index used by the integration tests. Update it only from a successful publication. Checkpoint/restore tests additionally require
a Linux host with the supported `runsc` configuration and capabilities.
A runnable fixture image alone does not establish those capabilities.

## Maintain the image

Only fixture maintainers build images. On a matching Linux Nix builder:

```sh
system=x86_64-linux # or aarch64-linux
nix build --no-update-lock-file \
  "path:.#packages.${system}.reprise-test-image" --out-link result-test-image
```

The native workload binaries can also be built on macOS with
`nix build path:.#reprise-test-tools`. This does not build a Linux image.

## Memory workload interface

Run these commands inside a sandbox through `SandboxService::execute`:

```sh
/bin/memory-state start
/bin/memory-state state
/bin/memory-state mutate "fresh-test-value"
```

Each successful command prints one JSON object:

```json
{"boot_nonce":"<64 lowercase hex characters>","value":null,"revision":0}
```

`start` spawns a new session, detaches all child stdio to `/dev/null`, and waits
up to 15 seconds for a private readiness pipe before exiting. Readiness is
reported only after the child binds `127.0.0.1:8765`; an existing listener causes
startup to fail rather than adopting its state. The child holds no Execution
stdio or pin. `tini` adopts it after the launcher exits.

The foreground `memory-state serve` command runs the same workload for manual
inspection. Its HTTP interface is `GET /state` and `POST /mutate` with
`{"value":"..."}`. Requests stay on sandbox loopback; no published port is
needed.

The server generates a cryptographically random 32-byte nonce at startup.
The nonce, value, and revision live only in that process's RAM. There is no
state file, environment seed, or replay log. A fresh process starts with a
different nonce, null value, and revision zero. Mutations increment the
revision; subsequent reads must report the same nonce.

## Extension point

Add small Go commands under `test-tools/cmd/` and list them in
`nix/reprise-test-tools.nix`; add other required packages declaratively to
`paths` in `nix/reprise-test-runtime.nix`. When Go dependencies change, update
`go.sum` and `vendorHash` together. Building the test image never depends on the
Rust workspace.
Additional test/base images can be separate Nix expressions when needed;
there is no base-image hierarchy.
