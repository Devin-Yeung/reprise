use std::time::Duration;

use reprise_api::Execute;
use serde_json::json;

#[test]
fn builder_constructs_the_complete_command() {
    let command = Execute::builder()
        .argv(["printf"])
        .arg("%s")
        .args([String::from("hello"), String::from("world")])
        .cwd("work")
        .env("MODE", "initial")
        .envs([("LANG", "C"), ("MODE", "bulk")])
        .env("MODE", "test")
        .stdin(b"input".to_vec())
        .timeout(Duration::from_secs(5))
        .build();

    insta::assert_json_snapshot!(json!({
        "argv": command.argv,
        "cwd": command.cwd,
        "env": command.env,
        "stdin": command.stdin,
        "timeout_ms": command.timeout.map(|timeout| timeout.as_millis()),
        "idempotency_key_is_none": command.idempotency_key.is_none(),
    }));
}
