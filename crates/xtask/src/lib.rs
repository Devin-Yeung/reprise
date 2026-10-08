//! Developer task runner for the Reprise workspace.
//!
//! Tasks run through cargo so the toolchain is the developer's own:
//!
//! ```text
//! cargo run -p xtask -- <task> [args...]
//! ```

pub mod integration;

/// Why a task could not run.
#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    /// No task name was given.
    #[error("usage: cargo run -p xtask -- <task> [args...]\ntasks: integration")]
    MissingTask,
    /// The task name is not one of the known tasks.
    #[error("unknown task: {task}\ntasks: integration")]
    UnknownTask { task: String },
    /// The task itself failed.
    #[error(transparent)]
    Task(#[from] integration::Error),
}

/// Dispatches `args` (task name and its arguments, without the binary name)
/// to a task and returns the process exit code.
pub fn run(args: &[String]) -> Result<std::process::ExitCode, TaskError> {
    let (task, rest) = match args.split_first() {
        None => return Err(TaskError::MissingTask),
        Some((task, rest)) => (task.as_str(), rest),
    };
    match task {
        "integration" => {
            let status = integration::run(rest)?;
            Ok(exit_code(status))
        }
        other => Err(TaskError::UnknownTask {
            task: other.to_owned(),
        }),
    }
}

fn exit_code(status: std::process::ExitStatus) -> std::process::ExitCode {
    match status.code() {
        Some(code) => std::process::ExitCode::from((code & 0xff) as u8),
        // Terminated by a signal; there is no exit code to forward.
        None => std::process::ExitCode::FAILURE,
    }
}
