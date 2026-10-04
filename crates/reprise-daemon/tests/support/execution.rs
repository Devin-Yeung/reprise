use anyhow::{Context, Result, bail, ensure};
use reprise_api::{Channel, Execute, ExecutionEvent, ExecutionState, SandboxId, SandboxService};
use std::time::Duration;
use tokio::time::{sleep, timeout};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

// Strict test primitive: no retries, partial output, or unknown outcomes.
// Return stdout only after exit code zero and all retained events are drained.
pub(super) async fn execute_successfully<S: SandboxService>(
    service: &S,
    sandbox: &SandboxId,
    argv: Vec<String>,
) -> Result<Vec<u8>> {
    timeout(COMMAND_TIMEOUT, async {
        let execution = service
            .execute(
                sandbox,
                Execute::builder()
                    .argv(argv)
                    .timeout(COMMAND_TIMEOUT)
                    .build(),
            )
            .await
            .context("execute")?;

        let mut cursor = 0;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        loop {
            let page = service
                .execution_events(&execution.id, cursor, 256)
                .await
                .context("execution_events")?;
            let empty = page.events.is_empty();

            for event in page.events {
                match event {
                    ExecutionEvent::Output { channel, chunk, .. } => match channel {
                        Channel::Stdout => stdout.extend(chunk),
                        Channel::Stderr => stderr.extend(chunk),
                    },
                    ExecutionEvent::Truncated { .. } => bail!("execution output was truncated"),
                    ExecutionEvent::Status { .. } => {}
                }
            }

            ensure!(
                stdout.len() + stderr.len() <= 65536,
                "unexpectedly large execution output"
            );
            cursor = page.next;

            let current = service
                .get_execution(&execution.id)
                .await
                .context("get_execution")?;

            ensure!(
                !current.output_truncated && !current.outcome_unknown,
                "execution output or outcome is incomplete: {current:?}"
            );

            match current.state {
                ExecutionState::Accepted | ExecutionState::Running => {}
                ExecutionState::Exited => {
                    ensure!(
                        current.exit_code == Some(0),
                        "command failed: {current:?}; stderr={}",
                        String::from_utf8_lossy(&stderr)
                    );

                    // Success requires complete output, not just a terminal Execution.
                    if empty {
                        return Ok(stdout);
                    }
                }
                _ => bail!(
                    "command execution failed: {current:?}; stderr={}",
                    String::from_utf8_lossy(&stderr)
                ),
            }

            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("command execution/output timed out")?
}
