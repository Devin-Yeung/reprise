//! Runs the runsc integration tests as root.
//!
//! The integration tests drive real containers, which needs privileges the
//! development host does not delegate to unprivileged users (cgroup
//! configuration, rootless uid mapping). CI runs its runners as root and
//! never needs this; locally the task builds the test binary as the
//! developer, then re-executes it under `sudo` with the environment the
//! tests require.
//!
//! The environment is resolved explicitly instead of relying on the test
//! support fallbacks (`runsc` from `PATH`, `./result` relative directory):
//! under `sudo`, `PATH` is the secure path and a relative `result` depends
//! on the working directory, so both are unreliable. Explicit store paths
//! behave identically for the user and for root.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

/// Environment variable the integration tests read the runsc executable from.
pub const RUNSC_ENV: &str = "REPRISE_RUNSC_BIN";
/// Environment variable the integration tests read the runtime artifact directory from.
pub const RUNTIME_ENV: &str = "REPRISE_TEST_RUNTIME";

const RUNSC_REF: &str = ".#runsc";
const RUNTIME_REF: &str = ".#reprise-test-runtime";

/// Why the integration task could not run.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not run cargo: {0}")]
    Cargo(#[from] std::io::Error),
    #[error("cargo did not report an executable for the reprise-runsc integration test target")]
    NoIntegrationBinary,
    #[error("`nix build {flake_ref}` failed")]
    Nix {
        flake_ref: &'static str,
        #[source]
        source: std::io::Error,
    },
}

/// A resolved integration run: which binary to execute, with what environment and
/// extra arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationPlan {
    /// The compiled integration test binary.
    pub executable: PathBuf,
    /// Environment variables set for the run, in insertion order.
    pub env: Vec<(String, OsString)>,
    /// Arguments passed through to the test binary (e.g. `--nocapture`).
    pub extra_args: Vec<String>,
}

/// Looks up process environment variables. [`std::env`] in production;
/// a fixed map in tests.
pub trait Environment {
    fn var(&self, name: &str) -> Option<OsString>;
}

/// Resolves a flake output reference (e.g. `.#runsc`) to its store path.
/// `nix build --print-out-paths` in production; a fixture in tests.
pub trait OutputResolver {
    fn resolve(&self, flake_ref: &str) -> std::io::Result<PathBuf>;
}

/// Reads `REPRISE_RUNSC_BIN` and `REPRISE_TEST_RUNTIME` from `environment`,
/// falling back to `outputs` for anything unset.
///
/// Precedence follows CI (`env REPRISE_...=... cargo test ...`): a set
/// environment variable always wins. `REPRISE_RUNSC_BIN` resolved through
/// nix is the package's `bin/runsc`.
pub fn plan(
    executable: PathBuf,
    extra_args: Vec<String>,
    environment: &dyn Environment,
    outputs: &dyn OutputResolver,
) -> Result<IntegrationPlan> {
    let runsc = match environment.var(RUNSC_ENV) {
        Some(value) => value,
        None => OsString::from(
            outputs
                .resolve(RUNSC_REF)
                .map_err(|source| Error::Nix {
                    flake_ref: RUNSC_REF,
                    source,
                })?
                .join("bin")
                .join("runsc"),
        ),
    };
    let runtime = match environment.var(RUNTIME_ENV) {
        Some(value) => value,
        None => OsString::from(outputs.resolve(RUNTIME_REF).map_err(|source| Error::Nix {
            flake_ref: RUNTIME_REF,
            source,
        })?),
    };
    Ok(IntegrationPlan {
        executable,
        env: vec![
            (RUNSC_ENV.to_owned(), runsc),
            (RUNTIME_ENV.to_owned(), runtime),
        ],
        extra_args,
    })
}

/// The full argv for the run: `sudo env NAME=VALUE ... executable args...`.
///
/// `sudo` itself is unconditional; running as root it is a no-op privilege
/// change, which keeps CI and local invocations on one code path.
pub fn sudo_argv(resolved: &IntegrationPlan) -> Vec<OsString> {
    let mut argv = vec![OsString::from("sudo"), OsString::from("env")];
    for (name, value) in &resolved.env {
        let mut entry = OsString::from(name.as_str());
        entry.push("=");
        entry.push(value);
        argv.push(entry);
    }
    argv.push(resolved.executable.as_os_str().to_owned());
    argv.extend(resolved.extra_args.iter().map(OsString::from));
    argv
}

/// Executes the plan with inherited stdio, so `sudo` can prompt and the
/// test binary's output streams live. Returns the test binary's exit status.
pub fn execute(resolved: &IntegrationPlan) -> std::io::Result<ExitStatus> {
    let mut argv = sudo_argv(resolved);
    let program = argv.remove(0);
    std::process::Command::new(program).args(&argv).status()
}

/// Extracts the integration test binary's path from `cargo test --no-run
/// --message-format=json` output lines.
///
/// Only the `reprise-runsc` test target named `integration` matches; build script
/// and other artifact messages are ignored. Matching is on the artifact's
/// manifest path, not the package id, whose format varies across cargo
/// versions.
pub fn executable_from_cargo_json(lines: impl Iterator<Item = String>) -> Result<PathBuf> {
    for line in lines {
        let Ok(message) = serde_json::from_str::<serde_json::Value>(&line) else {
            // Human-readable build progress, not a JSON message.
            continue;
        };
        // The package name is not matched against `package_id`: its format
        // varies across cargo versions (name prefix vs. URL fragment). The
        // manifest path identifies the crate reliably.
        let is_integration_target = message["reason"] == "compiler-artifact"
            && message["manifest_path"]
                .as_str()
                .is_some_and(|path| path.contains("/crates/reprise-runsc/"))
            && message["target"]["name"] == "integration"
            && message["target"]["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "test"));
        if !is_integration_target {
            continue;
        }
        return message["executable"]
            .as_str()
            .map(PathBuf::from)
            .ok_or(Error::NoIntegrationBinary);
    }
    Err(Error::NoIntegrationBinary)
}

/// Builds the integration test binary, resolves the environment, and executes the
/// plan. Returns the test binary's exit status.
pub fn run(extra_args: &[String]) -> Result<ExitStatus> {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("xtask lives in <workspace>/crates/xtask");

    let output = std::process::Command::new("cargo")
        .args([
            "test",
            "-p",
            "reprise-runsc",
            "--test",
            "integration",
            "--features",
            "integration-tests",
            "--no-run",
            "--message-format=json",
        ])
        .current_dir(&workspace_root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .output()?;
    let executable = executable_from_cargo_json(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_owned),
    )?;

    let resolved = plan(
        executable,
        extra_args.to_vec(),
        &SystemEnvironment,
        &NixOutputs {
            workspace_root: workspace_root.clone(),
        },
    )?;
    execute(&resolved).map_err(Error::Cargo)
}

/// [`Environment`] over the process environment.
struct SystemEnvironment;

impl Environment for SystemEnvironment {
    fn var(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name)
    }
}

/// [`OutputResolver`] backed by `nix build --print-out-paths`.
struct NixOutputs {
    workspace_root: PathBuf,
}

impl OutputResolver for NixOutputs {
    fn resolve(&self, flake_ref: &str) -> std::io::Result<PathBuf> {
        let output = std::process::Command::new("nix")
            .args(["build", flake_ref, "--no-link", "--print-out-paths"])
            .current_dir(&self.workspace_root)
            .stderr(std::process::Stdio::inherit())
            .output()?;
        if !output.status.success() {
            return Err(std::io::Error::other(format!(
                "nix build {flake_ref} exited with {}",
                output.status
            )));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let out_path = stdout.lines().next().unwrap_or_default().trim();
        PathBuf::from(out_path)
            .canonicalize()
            .map_err(|source| std::io::Error::other(format!("nix output path: {source}")))
    }
}

/// [`plan`]'s result type, re-exported for readability.
pub type Result<T> = std::result::Result<T, Error>;
