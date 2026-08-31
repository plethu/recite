#![cfg(unix)]

#[path = "support/stdio.rs"]
mod stdio;

use serde_json::json;
use stdio::{StdioHarness, file_uri};
use tempfile::Builder;

#[test]
fn symlink_schema_keeps_configured_uri_across_startup_overlay_and_close() {
    use std::os::unix::fs::symlink;

    let temp = Builder::new()
        .prefix("recite symlink schema stdio ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary schema directory: {error}"));
    let manifest = temp.path().join("recite.project.toml");
    let target = temp.path().join("target.json");
    let configured = temp.path().join("schema.json");
    std::fs::write(&target, "{\"schema_version\":\"bad\"}\n")
        .unwrap_or_else(|error| panic!("write target schema: {error}"));
    symlink(&target, &configured).unwrap_or_else(|error| panic!("symlink schema: {error}"));
    std::fs::write(
        &manifest,
        "format_version = 1\n[project]\nschema = \"schema.json\"\n",
    )
    .unwrap_or_else(|error| panic!("write schema manifest: {error}"));

    let configured_uri = file_uri(&configured);
    let target_uri = file_uri(&target);
    assert_ne!(configured_uri, target_uri);
    let mut harness = StdioHarness::start(json!({
        "capabilities": {},
        "rootUri": file_uri(temp.path())
    }));
    let startup = harness.barrier(&configured_uri);
    assert_eq!(startup.len(), 1, "startup schema messages: {startup:?}");
    assert_schema_message(&startup[0], &configured_uri, None, false);

    harness.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": target_uri.clone(),
                "languageId": "json",
                "version": 4,
                "text": "{\"schema_version\":1}\n"
            }
        }),
    );
    let opened = harness.barrier(&target_uri);
    assert_eq!(opened.len(), 1, "overlay schema messages: {opened:?}");
    assert_schema_message(&opened[0], &configured_uri, Some(4), true);

    harness.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": target_uri.clone() } }),
    );
    let closed = harness.barrier(&target_uri);
    assert_eq!(closed.len(), 1, "closed schema messages: {closed:?}");
    assert_schema_message(&closed[0], &configured_uri, None, false);
    assert!(
        closed[0]["params"]["uri"] != target_uri,
        "canonical target URI must not receive a stale publication: {closed:?}"
    );
    harness.finish();
}

fn assert_schema_message(
    message: &serde_json::Value,
    uri: &str,
    version: Option<i64>,
    empty: bool,
) {
    assert_eq!(message["method"], "textDocument/publishDiagnostics");
    assert_eq!(message["params"]["uri"], uri);
    assert_eq!(message["params"]["version"].as_i64(), version);
    assert_eq!(
        message["params"]["diagnostics"]
            .as_array()
            .is_some_and(Vec::is_empty),
        empty,
        "schema diagnostics payload: {message}"
    );
}
