use std::path::PathBuf;

use clap::Parser;
use reprise_daemon::cli::Cli;
use reprise_daemon::{DaemonConfig, DockerConfig, FixedTemplate, SnapshotStorageConfig};

#[test]
fn cli_accepts_a_forwarded_socket() {
    let cli = Cli::try_parse_from([
        "reprised",
        "--image",
        "alpine:3",
        "--docker-socket",
        "/tmp/lighthouse.sock",
    ])
    .unwrap();

    assert_eq!(cli.docker_socket, PathBuf::from("/tmp/lighthouse.sock"));
    assert_eq!(cli.runtime, "runsc");
    assert!(cli.listen.ip().is_loopback());
}

#[test]
fn configuration_is_constructible_with_typed_builders() {
    let config = DaemonConfig::builder()
        .listen("127.0.0.1:8080".parse().unwrap())
        .state_db("reprise.db")
        .snapshots(
            SnapshotStorageConfig::builder()
                .committed_dir("snapshots")
                .build(),
        )
        .docker(
            DockerConfig::builder()
                .socket("/tmp/lighthouse.sock")
                .runtime("runsc")
                .build(),
        )
        .default_template(
            FixedTemplate::builder()
                .id("default")
                .image("alpine:3")
                .build(),
        )
        .build();

    assert_eq!(config.docker.runtime, "runsc");
    assert_ne!(config.docker.socket, config.snapshots.committed_dir);
}

#[test]
fn binary_refuses_unimplemented_startup() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_reprised"))
        .args(["--image", "alpine:3"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("skeleton only")
    );
}
