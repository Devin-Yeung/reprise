//! Build the runsc integration tests as the developer, then run them as root.
//!
//! Explicit environment variables take precedence over Nix outputs. Passing them
//! through `sudo env` avoids depending on sudo's PATH or relative fallback paths.

mod cargo;

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use anyhow::{Context, Result, ensure};

const RUNSC_ENV: &str = "REPRISE_RUNSC_BIN";
const RUNTIME_ENV: &str = "REPRISE_TEST_RUNTIME";

/// Builds and runs the integration test binary with root privileges.
///
/// Arguments are forwarded to the test binary, and its exit status is returned.
pub fn run(extra_args: &[String]) -> Result<ExitStatus> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("xtask lives in <workspace>/crates/xtask");

    let executable = cargo::build_integration_test(workspace)?;
    let runsc = match std::env::var_os(RUNSC_ENV) {
        Some(value) => value,
        None => nix_output(workspace, ".#runsc")?
            .join("bin/runsc")
            .into_os_string(),
    };
    let runtime = match std::env::var_os(RUNTIME_ENV) {
        Some(value) => value,
        None => nix_output(workspace, ".#reprise-test-runtime")?.into_os_string(),
    };

    Command::new("sudo")
        .arg("env")
        .arg(env_assignment(RUNSC_ENV, &runsc))
        .arg(env_assignment(RUNTIME_ENV, &runtime))
        .arg(executable)
        .args(extra_args)
        .status()
        .context("could not run integration tests with sudo")
}

fn env_assignment(name: &str, value: &OsStr) -> OsString {
    let mut assignment = OsString::from(name);
    assignment.push("=");
    assignment.push(value);
    assignment
}

fn nix_output(workspace: &Path, flake_ref: &str) -> Result<PathBuf> {
    let output = Command::new("nix")
        .args(["build", flake_ref, "--no-link", "--print-out-paths"])
        .current_dir(workspace)
        .stderr(Stdio::inherit())
        .output()
        .with_context(|| format!("could not run nix build {flake_ref}"))?;
    ensure!(
        output.status.success(),
        "nix build {flake_ref} exited with {}",
        output.status
    );

    let stdout = String::from_utf8(output.stdout).context("nix output path is not UTF-8")?;
    let path = stdout.lines().next().unwrap_or_default().trim();
    ensure!(
        !path.is_empty(),
        "nix build {flake_ref} returned no output path"
    );
    Path::new(path)
        .canonicalize()
        .with_context(|| format!("invalid output path from nix build {flake_ref}: {path}"))
}
