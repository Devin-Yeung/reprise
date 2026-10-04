//! Opt-in acceptance test using the published test image. Only fixture setup
//! knows the Docker host. The scenario uses SandboxService exclusively, not
//! Docker commands or HTTP against the daemon. No fake service is supplied
//! while startup is missing.

mod support;

use anyhow::{Result, bail};
use reprise_daemon::{DaemonConfig, DockerConfig, FixedTemplate, SnapshotStorageConfig};
use reprise_test_support::TestFixture;

#[tokio::test]
#[ignore = "TODO: implement concrete daemon startup and sandbox suspend/resume"]
async fn background_http_state_survives_suspend_resume() -> Result<()> {
    let fixture = TestFixture::from_env().await?;
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
                .host(fixture.host)
                .runtime("runsc")
                .build(),
        )
        .default_template(
            FixedTemplate::builder()
                .id("default")
                .image(fixture.image.id)
                .build(),
        )
        .build();

    // Wiring point: start the concrete daemon with config, pass its service and
    // resolved TemplateId to support::verify, then shut it down. No mocked
    // SandboxService or direct Docker fallback may substitute for that path.
    bail!(
        "fixture startup not implemented: connect the concrete daemon to {} and run support::verify; no sandbox was created",
        config.docker.host
    )
}
