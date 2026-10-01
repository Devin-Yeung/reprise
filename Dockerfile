# syntax=docker/dockerfile:1
# Nix is needed only inside the Linux build stage, not on the host or at runtime.
FROM nixos/nix:2.28.5@sha256:e2bff159b88c242722022a273d3832a4392f01dba35b796204d852b89042526e AS builder
WORKDIR /src
COPY flake.nix flake.lock ./
COPY nix/ ./nix/
COPY test-tools/ ./test-tools/

RUN nix --extra-experimental-features "nix-command flakes" \
      build --no-update-lock-file --out-link /runtime path:.#reprise-test-runtime \
    && mkdir -p /rootfs/nix/store \
    && for store_path in $(nix-store --query --requisites /runtime); do \
         cp -a "$store_path" /rootfs/nix/store/; \
       done \
    && cp -a /runtime/. /rootfs/

FROM scratch
COPY --from=builder /rootfs/ /
# Keep runtime metadata in sync with nix/reprise-test-image.nix.
ENV PATH=/bin
WORKDIR /
ENTRYPOINT ["/bin/tini", "--"]
CMD ["/bin/sleep", "infinity"]
