use anyhow::{Context, Result, bail};
use reprise_api::{Operation, OperationState, SandboxService};
use std::time::Duration;
use tokio::time::{sleep, timeout};

const LIFECYCLE_TIMEOUT: Duration = Duration::from_secs(120);

pub(super) async fn wait_operation<S: SandboxService>(
    service: &S,
    mut operation: Operation,
) -> Result<Operation> {
    timeout(LIFECYCLE_TIMEOUT, async {
        loop {
            match operation.state {
                OperationState::Succeeded => return Ok(operation),
                OperationState::Failed => bail!("lifecycle operation failed: {operation:?}"),
                OperationState::Pending | OperationState::Running => {}
            }
            sleep(Duration::from_millis(100)).await;
            operation = service
                .get_operation(&operation.id)
                .await
                .context("get_operation")?;
        }
    })
    .await
    .context("lifecycle operation timed out")?
}
