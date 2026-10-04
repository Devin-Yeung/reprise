use anyhow::{Context, Result, bail, ensure};
use reprise_api::{OperationResult, Resume, SandboxId, SandboxService, SandboxState, Suspend};
use uuid::Uuid;

use super::{lifecycle::wait_operation, memory_state::MemoryWorkload};

pub(super) async fn continuity<S: SandboxService>(service: &S, sandbox: &SandboxId) -> Result<()> {
    let workload = MemoryWorkload::new(service, sandbox);
    let initial = workload.start().await?;

    ensure!(initial.boot_nonce.len() == 64, "invalid startup nonce");
    ensure!(
        initial.value.is_none() && initial.revision == 0,
        "service did not start empty"
    );

    // Generated AFTER startup, never part of the template or launcher.
    let value = Uuid::new_v4().to_string();
    let before = workload.mutate(&value).await?;

    ensure!(
        before.boot_nonce == initial.boot_nonce,
        "service restarted before checkpoint"
    );
    ensure!(
        before.value.as_deref() == Some(value.as_str()) && before.revision == 1,
        "first mutation was not applied"
    );

    let running = service
        .inspect(sandbox)
        .await
        .context("inspect")?;

    ensure!(
        running.sandbox.state == SandboxState::Running,
        "sandbox is not running"
    );
    ensure!(
        running.active_executions.is_empty(),
        "launcher or request still holds a pin"
    );

    let suspend = service
        .suspend(sandbox, Suspend::default())
        .await
        .context("suspend")?;
    let suspended = wait_operation(service, suspend).await?;

    let snapshot = match suspended.result {
        Some(OperationResult::Suspend { snapshot }) => snapshot,
        other => bail!("suspend did not commit a snapshot: {other:?}"),
    };

    // Do NOT execute here: wake-on-work would implicitly resume the sandbox.
    let info = service
        .inspect(sandbox)
        .await
        .context("inspect suspended")?;

    ensure!(
        info.sandbox.state == SandboxState::Suspended,
        "suspend did not release the runtime"
    );
    ensure!(
        info.sandbox.latest_snapshot.as_ref() == Some(&snapshot),
        "snapshot was not committed"
    );

    let resume = service
        .resume(sandbox, Resume::builder().build())
        .await
        .context("resume")?;
    let resumed = wait_operation(service, resume).await?;

    match resumed.result {
        Some(OperationResult::Resume {
            snapshot: Some(restored),
            generation,
        }) => {
            ensure!(restored == snapshot, "restored a different snapshot");
            ensure!(
                generation > running.sandbox.generation,
                "activation generation did not advance"
            );
        }
        other => bail!("resume cold-booted or did not report restoration: {other:?}"),
    }

    // No launcher or mutation replay between checkpoint and this read.
    let after = workload.read().await?;

    ensure!(
        after == before,
        "memory continuity lost: before={before:?}, after={after:?}"
    );

    // An additional mutation demonstrates a live server, not merely a cached
    // response from before suspend.
    let next_value = Uuid::new_v4().to_string();
    let next = workload.mutate(&next_value).await?;

    ensure!(
        next.boot_nonce == initial.boot_nonce,
        "restored service restarted"
    );
    ensure!(
        next.value.as_deref() == Some(next_value.as_str()) && next.revision == 2,
        "restored server did not accept a fresh mutation"
    );

    Ok(())
}
