//! A real saved image, one explicit Docker endpoint, and parallel test consumers.
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, ensure};
use bollard::Docker;
use bollard::models::{ContainerCreateBody, HostConfig};
use bollard::query_parameters::{
    CreateContainerOptions, RemoveContainerOptions, StartContainerOptions, WaitContainerOptions,
};
use futures_util::TryStreamExt;
use reprise_test_support::{DockerTestEnvironment, ImagePreparation};
use tokio::sync::Barrier;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires a saved test image and a Linux Docker endpoint with runsc"]
async fn loads_image_for_parallel_runsc_consumers() -> Result<()> {
    let socket = std::env::var("REPRISE_DOCKER_SOCKET")
        .context("set REPRISE_DOCKER_SOCKET to a Unix socket path")?;
    let archive = PathBuf::from(
        std::env::var_os("REPRISE_TEST_IMAGE_ARCHIVE")
            .context("set REPRISE_TEST_IMAGE_ARCHIVE to an uncompressed Docker save tar")?,
    );
    let environment =
        Arc::new(DockerTestEnvironment::connect(&socket, Duration::from_secs(120)).await?);
    let docker = Docker::connect_with_unix(&socket, 120, bollard::API_DEFAULT_VERSION)?
        .negotiate_version()
        .await?;
    let barrier = Arc::new(Barrier::new(4));
    let mut consumers = Vec::new();
    for _ in 0..4 {
        let (environment, docker, archive, barrier) = (
            environment.clone(),
            docker.clone(),
            archive.clone(),
            barrier.clone(),
        );
        consumers.push(tokio::spawn(async move {
            barrier.wait().await;
            let prepared = environment.ensure_image(&archive).await?;
            let image_id = prepared.image.id.context("prepared image has an ID")?;
            let cached = environment.ensure_image(&archive).await?;
            ensure!(cached.preparation == ImagePreparation::Cached);
            ensure!(cached.image.id.as_deref() == Some(image_id.as_str()));
            run_workload(&docker, &image_id).await?;
            Ok::<_, anyhow::Error>((image_id, prepared.preparation))
        }));
    }
    // Join every consumer before reporting errors, so sibling container cleanup
    // still runs. The CI endpoint starts empty; no test removes shared images.
    let mut results = Vec::new();
    for consumer in consumers {
        results.push(
            consumer
                .await
                .context("image consumer panicked")
                .and_then(|result| result),
        );
    }
    let results = results.into_iter().collect::<Result<Vec<_>>>()?;
    ensure!(
        results
            .iter()
            .any(|(_, preparation)| *preparation == ImagePreparation::Loaded),
        "use an endpoint without this image to exercise upload"
    );
    ensure!(
        results
            .iter()
            .all(|(image_id, _)| image_id == &results[0].0)
    );
    Ok(())
}

async fn run_workload(docker: &Docker, image_id: &str) -> Result<()> {
    // Docker allocates unique names/IDs; no shared port or host mount is needed.
    let container = docker
        .create_container(
            None::<CreateContainerOptions>,
            ContainerCreateBody {
                image: Some(image_id.to_owned()),
                cmd: Some(vec!["/bin/memory-state".into(), "start".into()]),
                host_config: Some(HostConfig {
                    runtime: Some("runsc".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await?;
    let result = tokio::time::timeout(Duration::from_secs(60), async {
        docker
            .start_container(&container.id, None::<StartContainerOptions>)
            .await?;
        let exit = docker
            .wait_container(&container.id, None::<WaitContainerOptions>)
            .try_next()
            .await?
            .context("missing container exit status")?;
        ensure!(
            exit.status_code == 0,
            "test workload exited with {}",
            exit.status_code
        );
        Ok::<_, anyhow::Error>(())
    })
    .await
    .context("runsc workload timed out")
    .and_then(|result| result);
    let cleanup = docker
        .remove_container(
            &container.id,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await;
    result?;
    cleanup.context("remove this consumer's container")?;
    Ok(())
}
