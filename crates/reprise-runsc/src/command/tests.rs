//! Expected argv follows gVisor d3d566ad1979a92cb76918a18cba6e51fe252393:
//! runsc/cli/cli.go, runsc/config/{flags,config}.go, and runsc/cmd/*.go.

use super::{Command, invocation};
use crate::{CreateOptions, GlobalOptions, Platform, RestoreOptions, RunscConfig};

/// Keeps one realistic restore path reviewable as input Rust values and output argv.
/// TODO: Add focused snapshots when another supported workflow needs its own contract.
#[test]
fn configured_restore() {
    let config = RunscConfig {
        executable: "/opt/gvisor/runsc".into(),
        state_root: "/var/lib/reprise/runsc".into(),
        options: GlobalOptions::builder().platform(Platform::Kvm).build(),
    };
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
