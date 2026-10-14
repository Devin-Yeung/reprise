//! Developer task runner for the Reprise workspace.
//!
//! Tasks run through cargo so the toolchain is the developer's own:
//!
//! ```text
//! cargo run -p xtask -- <task> [args...]
//! ```

pub mod integration;

use clap::{Parser, Subcommand};

/// Developer task runner for the Reprise workspace.
#[derive(Debug, Parser)]
#[command(name = "xtask", bin_name = "cargo run -p xtask --")]
pub struct Cli {
    #[command(subcommand)]
    pub task: Task,
}

/// A task runnable through `cargo run -p xtask -- <task>`.
#[derive(Debug, Subcommand)]
pub enum Task {
    /// Run the runsc integration tests as root.
    // `--help` belongs to the test binary, so clap must not intercept it here.
    #[command(disable_help_flag = true)]
    Integration {
        /// Arguments passed through to the test binary, e.g. `--nocapture`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

/// Runs the parsed task and returns the process exit code.
pub fn run(cli: Cli) -> anyhow::Result<std::process::ExitCode> {
    match cli.task {
        Task::Integration { args } => {
            let status = integration::run(&args)?;
            Ok(exit_code(status))
        }
    }
}

fn exit_code(status: std::process::ExitStatus) -> std::process::ExitCode {
    match status.code() {
        Some(code) => std::process::ExitCode::from((code & 0xff) as u8),
        // Terminated by a signal; there is no exit code to forward.
        None => std::process::ExitCode::FAILURE,
    }
}
