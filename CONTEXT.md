# Reprise

Reprise manages sandbox lifecycles and the work performed inside sandboxes.

## Language

**Sandbox**:
An isolated workspace and execution environment with a stable identity and lifecycle. That identity persists while suspended and across replacements of its running instance.
_Avoid_: Container, execution

**Snapshot**:
A saved state of a sandbox's processes and workspace that can be used to resume its execution. It does not include the state of external services or other independently managed resources.
_Avoid_: Memory dump, workspace backup

**Execution**:
One accepted request to run a command inside a sandbox, together with its output and outcome. It is distinct from a change to the sandbox's lifecycle.
_Avoid_: Operation, job

**Lifecycle operation**:
One accepted request to suspend, resume, or destroy a sandbox. It concerns the sandbox's lifecycle, not a command running inside it.
_Avoid_: Execution, unqualified operation
