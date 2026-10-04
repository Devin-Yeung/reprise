//! Opt-in acceptance test using a published test image. Only fixture setup
//! knows the Docker host; the image must already be present on that endpoint.
//! The scenario uses SandboxService exclusively, not Docker commands or HTTP
//! against the daemon. No fake service is supplied while startup is missing.

mod support;

use anyhow::{Context, Result, bail};
use reprise_daemon::{DaemonConfig, DockerConfig, FixedTemplate, SnapshotStorageConfig};

#[tokio::test]
#[ignore = "TODO: implement concrete daemon startup and sandbox suspend/resume"]
async fn background_http_state_survives_suspend_resume() -> Result<()> {
    let host = std::env::var("DOCKER_HOST").context(
        "set DOCKER_HOST to an explicit Docker endpoint URI; no default endpoint is used",
    )?;
    anyhow::ensure!(!host.trim().is_empty(), "DOCKER_HOST must not be empty");
    let image = std::env::var("REPRISE_TEST_IMAGE")
        .context("set REPRISE_TEST_IMAGE to the local image ID of the published fixture on the configured Docker endpoint")?;
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
        .docker(DockerConfig::builder().host(host).runtime("runsc").build())
        .default_template(FixedTemplate::builder().id("default").image(image).build())
        .build();

    // Wiring point: start the concrete daemon with config, pass its service and
    // resolved TemplateId to support::verify, then shut it down. No mocked
    // SandboxService or direct Docker fallback may substitute for that path.
    bail!(
        "fixture startup not implemented: connect the concrete daemon to {} and run support::verify; no Docker connection or sandbox creation was attempted",
        config.docker.host
    )
}
