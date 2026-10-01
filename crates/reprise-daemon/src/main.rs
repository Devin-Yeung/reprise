//! CLI entry point only; deliberately refuses to claim the daemon is running.

use std::process::ExitCode;

use clap::Parser;
use reprise_daemon::cli::Cli;

fn main() -> ExitCode {
    let _cli = Cli::parse();
    eprintln!("reprised: skeleton only; daemon startup is not implemented");
    ExitCode::FAILURE
}
