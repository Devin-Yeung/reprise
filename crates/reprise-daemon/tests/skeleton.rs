use clap::Parser;
use reprise_daemon::cli::Cli;
use reprise_daemon::{DaemonConfig, DockerConfig, FixedTemplate, SnapshotStorageConfig};

#[test]
fn cli_accepts_a_docker_host() {
    let cli = Cli::try_parse_from([
        "reprised",
        "--image",
        "alpine:3",
        "--docker-host",
        "unix:///tmp/lighthouse.sock",
    ])
    .unwrap();

    assert_eq!(cli.docker_host.as_str(), "unix:///tmp/lighthouse.sock");
    assert_eq!(cli.runtime, "runsc");
    assert!(cli.listen.ip().is_loopback());
}

#[test]
fn cli_accepts_a_remote_docker_host() {
    let cli = Cli::try_parse_from([
        "reprised",
        "--image",
        "alpine:3",
        "--docker-host",
        "ssh://ycg@workstation",
    ])
    .unwrap();

    assert_eq!(cli.docker_host.as_str(), "ssh://ycg@workstation");
}

#[test]
fn cli_leaves_https_connection_setup_to_bollard() {
    let cli = Cli::try_parse_from([
        "reprised",
        "--image",
        "alpine:3",
        "--docker-host",
        "https://workstation:2376",
    ])
    .unwrap();
    assert_eq!(cli.docker_host, "https://workstation:2376");
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
                .host("unix:///tmp/lighthouse.sock")
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
    assert_ne!(
        config.docker.host.as_str(),
        config.snapshots.committed_dir.to_str().unwrap()
    );
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
