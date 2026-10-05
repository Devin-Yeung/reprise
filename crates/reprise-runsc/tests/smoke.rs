#[allow(dead_code)]
mod support;

#[cfg(all(target_os = "linux", feature = "integration-tests"))]
mod tests {
    use std::io::Read;
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;
    use std::thread;
    use std::time::{Duration, Instant};

    use reprise_oci::Layer;
    use reprise_runsc::{
        ContainerId, ContainerIo, ContainerStatus, CreateOptions, DeleteOptions, ExecOptions,
        GlobalOptions, Network, OutputTarget, Runsc, RunscConfig,
    };
    use serde::Deserialize;

    use super::support::oci_bundle::PreparedBundle;

    #[derive(Deserialize)]
    struct MemoryState {
        boot_nonce: String,
        revision: u64,
    }

    fn runsc_binary() -> PathBuf {
        std::env::var_os("REPRISE_RUNSC_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("runsc"))
    }

    fn runsc(state_root: &tempfile::TempDir) -> Runsc {
        // Smoke tests must not access the host network. The OCI bundle provides
        // its own `/run` tmpfs when a workload needs loopback IPC.
        Runsc::new(
            RunscConfig::builder()
                .executable(runsc_binary())
                .state_root(state_root.path())
                .options(GlobalOptions::builder().network(Network::None).build())
                .build(),
        )
    }

    #[test]
    fn smoke_run_wait_and_delete() {
        let bundle = PreparedBundle::memory_state(&["version"], []);
        let state_root = tempfile::tempdir().expect("failed to create state root tempdir");
        let runsc = runsc(&state_root);

        // A socket pair lets this test observe workload output without sharing a
        // filesystem location with the sandbox. The launch options own the write
        // end; dropping them after launch closes the host copy, leaving only the
        // copies that runsc transferred into the sandbox.
        let (mut stdout_reader, stdout_writer) =
            UnixStream::pair().expect("failed to create stdout socket pair");
        let io = ContainerIo::builder()
            .stdout(OutputTarget::Fd(OwnedFd::from(stdout_writer)))
            .build();
        let create_options = CreateOptions::builder()
            .bundle(bundle.bundle_dir.path())
            .io(io)
            .build();
        let id = ContainerId::generate();

        // The short version command can exit before a separate `runsc wait`
        // invocation attaches. State is sufficient for this smoke test: it proves
        // the container reached its terminal lifecycle state without asserting an
        // exit code.
        runsc
            .run_detached(&id, &create_options)
            .expect("run_detached failed");
        drop(create_options);

        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let state = runsc.state(&id).expect("state failed");
            if state.status == ContainerStatus::Stopped {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "container did not stop before deadline"
            );
            thread::sleep(Duration::from_millis(10));
        }

        let mut stdout = String::new();
        stdout_reader
            .read_to_string(&mut stdout)
            .expect("failed to read stdout socket");
        assert!(
            stdout.contains("0.1.0"),
            "expected stdout to contain version 0.1.0, got {stdout:?}"
        );

        runsc
            .delete(&id, DeleteOptions { force: true })
            .expect("delete failed");
    }

    #[test]
    fn smoke_detached_server_answers_exec_get() {
        let bundle = PreparedBundle::memory_state(
            &["serve", "--socket", "/run/memory-state.sock"],
            [Layer::tmpfs("/run")],
        );
        let state_root = tempfile::tempdir().expect("failed to create state root tempdir");
        let runsc = runsc(&state_root);
        let create_options = CreateOptions::builder()
            .bundle(bundle.bundle_dir.path())
            .build();
        let id = ContainerId::generate();

        runsc
            .run_detached(&id, &create_options)
            .expect("run_detached failed");
        drop(create_options);

        let get_state = ExecOptions::builder()
            .program(&bundle.memory_state)
            .args(["get", "--socket", "/run/memory-state.sock"].map(Into::into))
            .build();
        let deadline = Instant::now() + Duration::from_secs(10);
        let state = loop {
            let output = runsc.exec(&id, &get_state).expect("exec get failed");
            if output.status.success() {
                break serde_json::from_slice::<MemoryState>(&output.stdout)
                    .unwrap_or_else(|err| panic!("exec get returned invalid JSON: {err}"));
            }
            assert!(
                Instant::now() < deadline,
                "memory-state did not become ready: {}",
                String::from_utf8_lossy(&output.stderr),
            );
            thread::sleep(Duration::from_millis(10));
        };

        assert_eq!(
            state.boot_nonce.len(),
            64,
            "server returned an invalid nonce"
        );
        assert_eq!(state.revision, 0, "get must not mutate the server state");

        runsc
            .delete(&id, DeleteOptions { force: true })
            .expect("delete failed");
    }
}
