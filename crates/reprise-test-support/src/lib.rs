//! Real Docker fixtures for opt-in integration tests.
//!
//! Select tests with `cargo test --features integration-tests`. Fixture startup
//! requires an explicit `DOCKER_HOST` URI and uses a pinned published fixture.
//! `REPRISE_TEST_IMAGE_REF` can override it with another `name@sha256:…` pin. It negotiates with
//! that Engine, pulls the reference if absent, and returns its endpoint-local
//! image ID. Missing configuration or unavailable infrastructure fails startup.
//! No Docker context, default socket, build, or alternative image is selected.

mod image;

use std::time::Duration;

use anyhow::{Context, Result, ensure};
use bollard::Docker;

pub use image::{ImagePreparation, PrepareError, PreparedImage, ensure_image};

/// A connected Engine and an immutable fixture image prepared on that endpoint.
///
/// Consumers can clone the client and reuse the image. Startup never removes or
/// retags shared images; each test must clean up the containers it creates.
/// Runtime and checkpoint support must be established by the consuming test.
pub struct TestFixture {
    /// Explicit endpoint URI, also usable when configuring the daemon.
    pub host: String,
    pub docker: Docker,
    /// The registry pin used for preparation, distinct from the local image ID.
    pub reference: String,
    pub image: PreparedImage,
}

impl TestFixture {
    /// Read `DOCKER_HOST`, connect, and prepare the pinned registry fixture.
    ///
    /// `REPRISE_TEST_IMAGE_REF` optionally overrides the checked-in image pin.
    /// Explicit values must be nonempty. Bollard interprets endpoint URIs,
    /// including Unix sockets, named pipes, TCP/TLS, and SSH on Unix; SSH needs
    /// local OpenSSH and remote `docker` on PATH. Docker contexts are not read.
    /// Connection/negotiation has a 30-second deadline, then image preparation
    /// has a separate 120-second deadline. An Engine pull may outlive cancellation.
    pub async fn from_env() -> Result<Self> {
        let host = required_env("DOCKER_HOST")?;
        let reference = match std::env::var_os("REPRISE_TEST_IMAGE_REF") {
            Some(_) => required_env("REPRISE_TEST_IMAGE_REF")?,
            None => include_str!("../test-image.ref").trim().to_owned(),
        };
        // A request deadline also bounds workload calls and cleanup performed
        // through this client. Stream consumers still need a whole-operation
        // deadline, since each message can otherwise restart the request clock.
        let docker = tokio::time::timeout(Duration::from_secs(30), async {
            Docker::connect_with_host(&host)?
                .with_timeout(Duration::from_secs(120))
                .negotiate_version()
                .await
        })
        .await
        .context("Docker connection deadline elapsed")?
        .with_context(|| format!("connect to Docker at {host}"))?;
        let image = ensure_image(&docker, &reference, Duration::from_secs(120))
            .await
            .with_context(|| format!("prepare fixture {reference} on {host}"))?;
        Ok(Self {
            host,
            docker,
            reference,
            image,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("set {name} for integration tests"))?;
    ensure!(!value.trim().is_empty(), "{name} must not be empty");
    Ok(value)
}
