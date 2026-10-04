use anyhow::{Context, Result, bail};
use reprise_api::{Operation, OperationResult, OperationState, SandboxService};
use std::time::Duration;
use tokio::time::{sleep, timeout};

const LIFECYCLE_TIMEOUT: Duration = Duration::from_secs(120);
const POLL_INTERVAL: Duration = Duration::from_millis(100);

pub(super) async fn wait_operation<S: SandboxService>(
    service: &S,
    mut operation: Operation,
) -> Result<OperationResult> {
    timeout(LIFECYCLE_TIMEOUT, async {
        loop {
            match operation.state {
                OperationState::Succeeded(result) => return Ok(result),
                OperationState::Failed(error) => {
                    bail!("lifecycle operation {} failed: {error}", operation.id)
                }
                OperationState::Pending | OperationState::Running => {}
            }
            sleep(POLL_INTERVAL).await;
            operation = service
                .get_operation(&operation.id)
                .await
                .context("get_operation")?;
        }
    })
    .await
    .context("lifecycle operation timed out")?
}
