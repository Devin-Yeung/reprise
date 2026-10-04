mod continuity;
mod execution;
mod lifecycle;
mod memory_state;

use anyhow::{Context, Result, ensure};
use reprise_api::{Capability, CreateSandbox, Destroy, SandboxService, TemplateId};
use std::time::Duration;
use tokio::time::timeout;

use continuity::continuity;
use lifecycle::wait_operation;

// This generic scenario is deliberately compiled before a concrete daemon
// exists. Remove the expectation when the fixture calls it.
#[expect(
    dead_code,
    reason = "the concrete daemon fixture is not implemented yet"
)]
pub async fn verify<S: SandboxService>(service: &S, template: TemplateId) -> Result<()> {
    let capabilities = service.capabilities().await;
    ensure!(
        capabilities.supports(Capability::ProcessCheckpoint),
        "process checkpoint capability is unproven"
    );
    ensure!(
        capabilities.supports(Capability::SameContainerRestore),
        "same-container restore capability is unproven"
    );

    let sandbox = service
        .create(CreateSandbox::builder().template(template).build())
        .await
        .context("create")?;

    // Return errors rather than panic so teardown is attempted on failed
    // assertions and timeout. Only this test's SandboxId is ever destroyed.
    let outcome = timeout(Duration::from_secs(300), continuity(service, &sandbox.id)).await;

    let cleanup = timeout(Duration::from_secs(30), async {
        let operation = service
            .destroy(&sandbox.id, Destroy::builder().force(true).build())
            .await
            .context("destroy")?;
        wait_operation(service, operation).await?;
        Ok::<_, anyhow::Error>(())
    })
    .await;

    let outcome = outcome
        .context("continuity scenario timed out")
        .and_then(|result| result);

    if let Err(error) = outcome {
        return Err(error.context(format!("cleanup result: {cleanup:?}")));
    }

    cleanup.context("cleanup timed out")??;

    Ok(())
}
