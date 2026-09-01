mod support;

use serde_json::{Value, json};
use std::path::Path;
use support::stdio::{StdioHarness, file_uri};
use tempfile::Builder;

#[test]
fn failed_workspace_folders_isolate_open_source_partitions() {
    let temp = Builder::new()
        .prefix("recite % stdio failed workspace ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary workspace: {error}"));
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    for root in [&first, &second] {
        std::fs::create_dir_all(root)
            .unwrap_or_else(|error| panic!("create failed workspace root: {error}"));
        std::fs::write(root.join("recite.project.toml"), "format_version = [\n")
            .unwrap_or_else(|error| panic!("write failed workspace manifest: {error}"));
    }
    let first_manifest_uri = file_uri(&first.join("recite.project.toml"));
    let second_manifest_uri = file_uri(&second.join("recite.project.toml"));
    let first_uri = file_uri(&first.join("src/live.recite"));
    let second_uri = file_uri(&second.join("src/live.recite"));
    let mut harness = StdioHarness::start(json!({
        "capabilities": {},
        "workspaceFolders": workspace_folders(&[&first, &second])
    }));
    let first_startup = harness.receive_message();
    let second_startup = harness.receive_message();
    assert_eq!(first_startup["params"]["uri"], first_manifest_uri);
    assert_eq!(second_startup["params"]["uri"], second_manifest_uri);

    let source = "oops\n:: shared default\n";
    harness.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": first_uri.clone(),
                "languageId": "recite",
                "version": 1,
                "text": source
            }
        }),
    );
    let first_messages = harness.barrier(&first_uri);
    assert_published_source_only(&first_messages, &first_uri, 1);

    harness.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": second_uri.clone(),
                "languageId": "recite",
                "version": 1,
                "text": source
            }
        }),
    );
    let second_messages = harness.barrier(&second_uri);
    assert_published_source_only(&second_messages, &first_uri, 1);
    assert_published_source_only(&second_messages, &second_uri, 1);

    std::fs::create_dir_all(first.join("src"))
        .unwrap_or_else(|error| panic!("create recovered source root: {error}"));
    std::fs::write(
        first.join("recite.project.toml"),
        "format_version = 1\n[discovery]\nsource_roots = [\"src\"]\n",
    )
    .unwrap_or_else(|error| panic!("write recovered workspace manifest: {error}"));
    std::fs::write(first.join("src/live.recite"), ":: saved\n")
        .unwrap_or_else(|error| panic!("write recovered source: {error}"));
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": first_manifest_uri, "type": 2 }] }),
    );
    let recovered_messages = harness.barrier(&first_uri);
    assert_published_source_only(&recovered_messages, &first_uri, 1);
    assert!(
        recovered_messages.iter().any(|message| {
            message["method"] == "textDocument/publishDiagnostics"
                && message["params"]["uri"] == second_uri
        }),
        "unaffected failed root should retain its own diagnostics: {recovered_messages:?}"
    );
    harness.finish();
}

fn assert_published_source_only(messages: &[Value], uri: &str, version: i64) {
    let published = messages
        .iter()
        .filter(|message| {
            message["method"] == "textDocument/publishDiagnostics"
                && message["params"]["uri"] == uri
        })
        .collect::<Vec<_>>();
    assert_eq!(published.len(), 1, "diagnostics for {uri}: {messages:?}");
    let diagnostics = &published[0]["params"];
    assert_eq!(diagnostics["version"], version);
    assert!(
        !diagnostics["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("diagnostics array is missing: {diagnostics}"))
            .is_empty()
    );
    assert!(!diagnostics["diagnostics"].as_array().is_some_and(|items| {
        items
            .iter()
            .any(|diagnostic| diagnostic["code"] == "RECITE_VALIDATE009")
    }));
}

fn workspace_folders(paths: &[&Path]) -> Vec<Value> {
    paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            json!({
                "uri": file_uri(path),
                "name": format!("workspace-{index}")
            })
        })
        .collect()
}
