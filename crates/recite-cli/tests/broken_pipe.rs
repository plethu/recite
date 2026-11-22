//! Piping output into a reader that stops early, as `| head` does, ends the command quietly.
use std::path::Path;
use std::process::{Command, Stdio};

#[test]
fn closed_stdout_exits_quietly() {
    let schema = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/realistic/v1-pack/schema/realistic.schema.json");
    for args in [
        vec![
            "schema".into(),
            "inspect".into(),
            schema.display().to_string(),
        ],
        vec!["explain".into(), "RECITE_PARSE001".into()],
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_recite"))
            .args(&args)
            .env_remove("RECITE_CONFIG")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn recite");
        drop(child.stdout.take());
        let output = child.wait_with_output().expect("wait for recite");
        assert!(output.status.success(), "{args:?}: {output:?}");
        assert!(
            output.stderr.is_empty(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
