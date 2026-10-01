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

## Build and load

`flake.lock` pins nixpkgs, including the Go toolchain and runtime tools.
`go.mod` / `go.sum` lock Go dependencies; Nix also hashes the vendored closure.
`nix/reprise-test-tools.nix` builds only `test-tools/`, with CGO disabled.
`nix/reprise-test-image.nix` assembles their runtime closures from scratch with
a fixed creation timestamp and a Nix-output-derived image tag, not `latest`.
The Nix-built runtime has no upstream base-image dependency.

Build for the **Docker host's** architecture, not necessarily the developer's:

```sh
target_system=x86_64-linux # or aarch64-linux
image_attr="path:.#packages.${target_system}.reprise-test-image"
nix build "$image_attr" --out-link result-test-image
tag=$(nix eval --raw "${image_attr}.imageTag")
```

Linux image builds require a matching Linux Nix builder. On macOS, configure
a Linux remote builder or run the build on Linux; a Docker socket tunnel does
not itself provide a Nix builder. The native Go tools can also be built on
Apple Silicon macOS with `nix build path:.#reprise-test-tools`.

### Docker-only build

No host Nix installation or remote Nix builder is needed for this path.
The Dockerfile runs Nix inside a Linux builder container, then copies the shared
runtime layout and its complete Nix store closure into a `scratch` image.
The final image contains neither Nix nor the builder's operating system.

Build on the **same explicit endpoint** the acceptance fixture will use:

```sh
export REPRISE_DOCKER_SOCKET=/tmp/reprise-lighthouse.sock
docker --host "unix://$REPRISE_DOCKER_SOCKET" build \
  --platform linux/amd64 \
  --tag reprise-test-image:local .
export REPRISE_TEST_IMAGE=$(
  docker --host "unix://$REPRISE_DOCKER_SOCKET" image inspect \
    --format '{{.Id}}' reprise-test-image:local
)
```

Use `linux/arm64` for an ARM64 Docker host. Cross-architecture builds require
Docker's emulation support; native builds do not. Docker Desktop provides
the Linux build environment on macOS. Build-time network access is required
for the Nix builder image and dependencies; no Docker socket is mounted into
the build.

Both paths use `nix/reprise-test-runtime.nix` and the locked dependencies.
The Dockerfile mirrors the entrypoint, command, and environment from
`nix/reprise-test-image.nix`; keep these synchronized. Docker's image metadata
and layer assembly differ from `dockerTools`, so the two paths need not produce
the same image ID. The builder's multi-architecture `nixos/nix` image is pinned
by digest; Docker's output still is not promised to be byte-for-byte reproducible.
The `local` tag is only a build label; pass the immutable image ID to tests.

### Load a Nix-built image

Load into the **same explicit endpoint** the acceptance fixture will use:

```sh
export REPRISE_DOCKER_SOCKET=/tmp/reprise-lighthouse.sock
docker --host "unix://$REPRISE_DOCKER_SOCKET" image load --input result-test-image
export REPRISE_TEST_IMAGE=$(
  docker --host "unix://$REPRISE_DOCKER_SOCKET" image inspect \
    --format '{{.Id}}' "reprise-test-image:$tag"
)
```

The image ID selects the loaded artifact immutably. Loading the image is
external fixture preparation, not part of the `SandboxService` scenario.
There is no registry push or implicit image pull.

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
there is no base-image hierarchy or image registry abstraction yet.
