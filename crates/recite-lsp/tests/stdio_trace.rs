mod support;

use serde_json::{Value, json};
use support::stdio::StdioHarness;

#[test]
fn native_trace_preserves_protocol_and_excludes_document_content() {
    let directory = tempfile::tempdir().unwrap();
    let mut server = StdioHarness::start_uninitialized_with_env(
        json!({"capabilities": {}}),
        &[("RECITE_LSP_TRACE_DIR", directory.path().as_os_str())],
    );
    server.send_initialized();
    let uri = "file:///workspace/private.recite";
    server.did_open(uri, 1, ":: private_block\n-> END\n");
    assert!(
        server.diagnostics(uri)["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let id = server.request(
        "textDocument/completion",
        json!({
            "textDocument": {"uri": uri}, "position": {"line": 1, "character": 3}
        }),
    );
    assert!(server.response_message(id).get("result").is_some());
    server.finish();
    let files: Vec<_> = std::fs::read_dir(directory.path()).unwrap().collect();
    assert_eq!(files.len(), 1);
    let trace = std::fs::read_to_string(files[0].as_ref().unwrap().path()).unwrap();
    assert!(!trace.contains("private"));
    let events: Vec<Value> = trace
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    for phase in [
        "ingress",
        "queued",
        "query_dispatch",
        "query_start",
        "query_end",
        "query_observed",
        "output_ready",
        "handoff",
    ] {
        assert!(
            events.iter().any(|event| event["fields"]["phase"] == phase),
            "missing {phase}"
        );
    }
}

#[test]
fn invalid_trace_destination_fails_without_writing_protocol_output() {
    let directory = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_recite-lsp"))
        .env("RECITE_LSP_TRACE_DIR", directory.path().join("missing"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot configure native trace"));
}
