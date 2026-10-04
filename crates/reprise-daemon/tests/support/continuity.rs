use anyhow::{Context, Result, bail, ensure};
use reprise_api::{OperationResult, Resume, SandboxId, SandboxService, SandboxState, Suspend};
use reprise_test_support::memory_state::MemoryState;
use uuid::Uuid;

use super::{ensure_eq, lifecycle::wait_operation, memory_state::MemoryWorkload};

pub(super) async fn continuity<S: SandboxService>(service: &S, sandbox: &SandboxId) -> Result<()> {
    let workload = MemoryWorkload::new(service, sandbox);
    let initial = workload.start().await?;

    ensure!(
        initial.has_valid_boot_nonce(),
        "invalid startup nonce: {initial:?}"
    );
    let empty = MemoryState {
        boot_nonce: initial.boot_nonce.clone(),
        value: None,
        revision: 0,
    };
    ensure_eq!(initial, empty, "service did not start empty");

    // Generated AFTER startup, never part of the template or launcher.
    let value = Uuid::new_v4().to_string();
    let before = workload.mutate(&value).await?;

    // A changed nonce means the service restarted before checkpoint.
    let expected = MemoryState {
        value: Some(value),
        revision: 1,
        ..empty.clone()
    };
    ensure_eq!(before, expected, "first mutation was not applied");

    let running = service.inspect(sandbox).await.context("inspect")?;

    ensure_eq!(
        running.sandbox.state,
        SandboxState::Running,
        "sandbox is not running"
    );
    ensure!(
        running.active_executions.is_empty(),
        "launcher or request still holds a pin: {:?}",
        running.active_executions
    );

    let suspend = service
        .suspend(sandbox, Suspend::builder().build())
        .await
        .context("suspend")?;
    let suspended = wait_operation(service, suspend).await?;

    let snapshot = match suspended {
        OperationResult::Suspend { snapshot } => snapshot,
        other => bail!("suspend did not commit a snapshot: {other:?}"),
    };

    // Do NOT execute here: wake-on-work would implicitly resume the sandbox.
    let info = service
        .inspect(sandbox)
        .await
        .context("inspect suspended")?;

    ensure_eq!(
        info.sandbox.state,
        SandboxState::Suspended,
        "suspend did not release the runtime"
    );
    ensure_eq!(
        info.sandbox.latest_snapshot,
        Some(snapshot.clone()),
        "snapshot was not committed"
    );

    let resume = service
        .resume(sandbox, Resume::builder().build())
        .await
        .context("resume")?;
    let resumed = wait_operation(service, resume).await?;

    match resumed {
        OperationResult::Resume {
            snapshot: Some(restored),
            generation,
        } => {
            ensure_eq!(restored, snapshot, "restored a different snapshot");
            ensure!(
                generation > running.sandbox.generation,
                "activation generation did not advance: {generation} <= {}",
                running.sandbox.generation
            );
        }
        other => bail!("resume cold-booted or did not report restoration: {other:?}"),
    }

    // No launcher or mutation replay between checkpoint and this read.
    let after = workload.read().await?;

    ensure_eq!(after, before, "memory continuity lost");

    // An additional mutation demonstrates a live server, not merely a cached
    // response from before suspend.
    let next_value = Uuid::new_v4().to_string();
    let next = workload.mutate(&next_value).await?;

    let expected = MemoryState {
        value: Some(next_value),
        revision: 2,
        ..empty
    };
    ensure_eq!(
        next,
        expected,
        "restored server did not accept a fresh mutation"
    );

    Ok(())
}
