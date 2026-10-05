//! Expected argv follows gVisor d3d566ad1979a92cb76918a18cba6e51fe252393:
//! runsc/cli/cli.go, runsc/config/{flags,config}.go, and runsc/cmd/*.go.

use std::ffi::OsString;

use super::{Command, invocation};
use crate::{
    CreateOptions, ExecOptions, GlobalOptions, Network, Platform, RestoreOptions, RunscConfig,
};

#[test]
fn exec_keeps_the_container_id_before_the_program_and_arguments() {
    // `runsc exec` addresses a running instance before naming the transient
    // process. Keeping that order explicit prevents a program path from ever
    // being interpreted as another container identifier.
    let config = RunscConfig::builder()
        .executable("/opt/gvisor/runsc")
        .state_root("/var/lib/reprise/runsc")
        .options(GlobalOptions::builder().network(Network::None).build())
        .build();
    let options = ExecOptions::builder()
        .program("/nix/store/test-tools/bin/memory-state")
        .args(["get", "--socket", "/run/memory-state.sock"].map(Into::into))
        .build();

    let invocation = invocation(
        &config,
        Command::Exec {
            id: "snapshot-check",
            options: &options,
        },
    );

    assert_eq!(
        invocation.args,
        [
            "--root",
            "/var/lib/reprise/runsc",
            "--network=none",
            "exec",
            "snapshot-check",
            "/nix/store/test-tools/bin/memory-state",
            "get",
            "--socket",
            "/run/memory-state.sock",
        ]
        .map(OsString::from),
    );
}

/// Keeps one realistic restore path reviewable as input Rust values and output argv.
/// TODO: Add focused snapshots when another supported workflow needs its own contract.
#[test]
fn configured_restore() {
    let config = RunscConfig::builder()
        .executable("/opt/gvisor/runsc")
        .state_root("/var/lib/reprise/runsc")
        .options(GlobalOptions::builder().platform(Platform::Kvm).build())
        .build();
    let options = RestoreOptions::builder()
        .create(
            CreateOptions::builder()
                .bundle("/var/lib/reprise/bundle")
                .build(),
        )
        .image_path("/var/lib/reprise/checkpoint")
        .background(true)
        .direct_io(true)
        .build();
    let command = Command::Restore {
        id: "benchmark-1",
        options: &options,
    };

    // Keeping the typed request next to argv makes each emitted flag traceable
    // to the configuration value that asked for it.
    let input = format!("RunscConfig:\n{config:#?}\n\nCommand:\n{command:#?}");
    let actual = invocation(&config, command);
    insta::assert_snapshot!(
        "configured_restore",
        format!("{input}\n\nInvocation:\n{actual:#?}")
    );
}
