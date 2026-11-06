use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow, ensure};
use cargo_metadata::Message;

pub(super) fn build_integration_test(workspace: &Path) -> Result<PathBuf> {
    let output = Command::new("cargo")
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
        .current_dir(workspace)
        .stderr(Stdio::inherit())
        .output()
        .context("could not run cargo test --no-run")?;

    let manifest = workspace.join("crates/reprise-runsc/Cargo.toml");
    let mut executable = None;
    for message in Message::parse_stream(Cursor::new(output.stdout)) {
        match message.context("could not read Cargo output")? {
            Message::CompilerArtifact(artifact)
                if artifact.manifest_path.as_std_path() == manifest
                    && artifact.target.name == "integration"
                    && artifact.target.is_test()
                    && executable.is_none() =>
            {
                executable = artifact.executable.map(|path| path.into_std_path_buf());
            }
            Message::CompilerMessage(message) => {
                if let Some(rendered) = &message.message.rendered {
                    eprint!("{rendered}");
                } else {
                    eprintln!("{}", message.message.message);
                }
            }
            _ => {}
        }
    }
    ensure!(
        output.status.success(),
        "cargo test --no-run exited with {}",
        output.status
    );
    executable.ok_or_else(|| {
        anyhow!("cargo did not report an executable for the reprise-runsc integration test target")
    })
}
