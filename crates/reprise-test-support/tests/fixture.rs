//! Real fixture workloads, independent of the unfinished daemon lifecycle.
//! Each workload owns its container; shared images are never removed.

use std::time::Duration;

use anyhow::{Context, Result, ensure};
use bollard::Docker;
use bollard::models::{ContainerCreateBody, HostConfig};
use bollard::query_parameters::{
    CreateContainerOptions, LogsOptionsBuilder, RemoveContainerOptionsBuilder,
    StartContainerOptions, WaitContainerOptions,
};
use futures_util::TryStreamExt;
use reprise_test_support::TestFixture;
use reprise_test_support::memory_state::{MEMORY_STATE, MemoryState};

const DEADLINE: Duration = Duration::from_secs(120);

#[tokio::test]
async fn fixture_workloads_are_isolated_under_runsc() -> Result<()> {
    let fixture = TestFixture::from_env().await?;
    // Both workloads finish cleanup even when one fails. A missing runsc
    // runtime is an infrastructure failure, never a reason to skip this test.
    let (first, second) = tokio::join!(
        run_workload(&fixture.docker, &fixture.image.id, "alpha"),
        run_workload(&fixture.docker, &fixture.image.id, "bravo"),
    );
    ensure!(first? != second?, "isolated workloads reused a boot nonce");
    Ok(())
}

async fn run_workload(docker: &Docker, image_id: &str, value: &str) -> Result<String> {
    let container = docker.create_container(
        None::<CreateContainerOptions>,
        ContainerCreateBody {
            image: Some(image_id.to_owned()),
            cmd: Some(vec![
                "/bin/sh".into(), "-c".into(),
                format!("{MEMORY_STATE} start && {MEMORY_STATE} mutate {value} && {MEMORY_STATE} state"),
            ]),
            host_config: Some(HostConfig { runtime: Some("runsc".into()), ..Default::default() }),
            ..Default::default()
        },
    ).await?;
    let id = container.id;
    let outcome = tokio::time::timeout(DEADLINE, async {
        docker
            .start_container(&id, None::<StartContainerOptions>)
            .await?;
        let exit = docker
            .wait_container(&id, None::<WaitContainerOptions>)
            .try_next()
            .await?
            .context("Docker did not report a workload exit")?;
        let mut output = Vec::new();
        let mut logs = docker.logs(
            &id,
            Some(
                LogsOptionsBuilder::default()
                    .stdout(true)
                    .stderr(true)
                    .build(),
            ),
        );
        while let Some(chunk) = logs.try_next().await? {
            output.extend_from_slice(&chunk.into_bytes());
        }
        ensure!(
            exit.status_code == 0,
            "workload exited with {}: {}",
            exit.status_code,
            String::from_utf8_lossy(&output)
        );
        Ok::<_, anyhow::Error>(output)
    })
    .await;

    let cleanup = docker
        .remove_container(
            &id,
            Some(RemoveContainerOptionsBuilder::default().force(true).build()),
        )
        .await;
    if let Err(error) = &cleanup {
        eprintln!("cleanup of workload container {id} failed: {error}");
    }
    let output = outcome.context("fixture workload deadline elapsed")??;
    cleanup?;
    let output = String::from_utf8(output)?;
    let states: Vec<MemoryState> = output
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    ensure!(
        states.len() == 3,
        "expected start/mutate/state output: {output}"
    );
    let [start, mutated, current] = states.as_slice() else {
        unreachable!()
    };
    ensure!(
        start.value.is_none() && start.revision == 0,
        "workload did not start fresh"
    );
    ensure!(start.has_valid_boot_nonce(), "invalid boot nonce");
    ensure!(
        mutated.boot_nonce == start.boot_nonce,
        "mutation changed boot identity"
    );
    ensure!(
        mutated.value.as_deref() == Some(value) && mutated.revision == 1,
        "mutation was not retained"
    );
    ensure!(current == mutated, "state read did not retain the mutation");
    Ok(start.boot_nonce.clone())
}
