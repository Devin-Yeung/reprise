//! Opt-in acceptance test using a prebuilt Nix test image. Only fixture setup
//! knows the Docker socket; the image must already be loaded on that endpoint.
//! The scenario uses SandboxService exclusively, not Docker commands or HTTP
//! against the daemon. No fake service is supplied while startup is missing.

mod support;

use anyhow::{Context, Result, bail};
use reprise_daemon::{DaemonConfig, DockerConfig, FixedTemplate, SnapshotStorageConfig};
use std::path::PathBuf;

#[tokio::test]
#[ignore = "requires REPRISE_DOCKER_SOCKET, REPRISE_TEST_IMAGE, runsc checkpoint support, and a concrete daemon"]
async fn background_http_state_survives_suspend_resume() -> Result<()> {
    let socket = std::env::var_os("REPRISE_DOCKER_SOCKET")
        .context("set REPRISE_DOCKER_SOCKET to an explicit Unix socket path; no default Docker endpoint is used")?;
    let socket = PathBuf::from(socket);
    anyhow::ensure!(
        socket.is_absolute(),
        "REPRISE_DOCKER_SOCKET must be an absolute path"
    );
    anyhow::ensure!(
        socket.exists(),
        "Docker socket path does not exist: {}",
        socket.display()
    );
    let image = std::env::var("REPRISE_TEST_IMAGE")
        .context("set REPRISE_TEST_IMAGE to the Nix-built reprise-test-image reference loaded on the configured Docker endpoint")?;
    anyhow::ensure!(
        !image.trim().is_empty(),
        "REPRISE_TEST_IMAGE must not be empty"
    );
    let state = tempfile::tempdir()?;
    let config = DaemonConfig::builder()
        .listen("127.0.0.1:0".parse()?)
        .state_db(state.path().join("state.db"))
        .snapshots(
            SnapshotStorageConfig::builder()
                .committed_dir(state.path().join("snapshots"))
                .build(),
        )
        .docker(
            DockerConfig::builder()
                .socket(socket)
                .runtime("runsc".to_owned())
                .build(),
        )
        .default_template(
            FixedTemplate::builder()
                .id("default".to_owned())
                .image(image)
                .build(),
        )
        .build();

    // Wiring point: start the concrete daemon with config, pass its service and
    // resolved TemplateId to support::verify, then shut it down. No mocked
    // SandboxService or direct Docker fallback may substitute for that path.
    bail!(
        "fixture startup not implemented: connect the concrete daemon to {} and run support::verify; no Docker connection or sandbox creation was attempted",
        config.docker.socket.display()
    )
}
