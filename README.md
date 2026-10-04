# **Reprise** —  Pause an environment. Resume its state.


Run ordinary development checks without Docker:

```sh
cargo test --locked --workspace --all-targets
```

Docker integration tests are selected explicitly:

```sh
DOCKER_HOST=ssh://user@workstation \
  cargo test --locked --workspace --all-targets --features integration-tests
```

The target Engine must support `runsc`; missing configuration or infrastructure
fails the selected tests. See [test fixtures](docs/design/test-support-crate.md)
for registry-only checks and CI configuration.
