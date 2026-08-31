mod support;

use std::path::Path;

use serde_json::{Value, json};
use support::stdio::{StdioHarness, file_uri};
use tempfile::Builder;

#[test]
fn stdio_workspace_folders_keep_manifest_and_fallback_saved_documents() {
    let temp = Builder::new()
        .prefix("recite % stdio workspace ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary workspace: {error}"));
    let manifest_root = temp.path().join("project");
    let fallback_root = temp.path().join("standalone");
    std::fs::create_dir_all(&manifest_root)
        .unwrap_or_else(|error| panic!("create project root: {error}"));
    std::fs::write(
        manifest_root.join("recite.project.toml"),
        "format_version = 1\n[discovery]\nsource_roots = [\"dialogue\"]\n",
    )
    .unwrap_or_else(|error| panic!("write project manifest: {error}"));
    std::fs::create_dir_all(manifest_root.join("dialogue"))
        .unwrap_or_else(|error| panic!("create dialogue root: {error}"));
    std::fs::write(
        manifest_root.join("dialogue/project.recite"),
        ":: project\n",
    )
    .unwrap_or_else(|error| panic!("write manifest source: {error}"));
    std::fs::create_dir_all(&fallback_root)
        .unwrap_or_else(|error| panic!("create fallback root: {error}"));
    let fallback = fallback_root.join("standalone.recite");
    std::fs::write(&fallback, ":: standalone\n")
        .unwrap_or_else(|error| panic!("write fallback source: {error}"));

    let fallback_uri = file_uri(&fallback);
    let mut harness = StdioHarness::start(json!({
        "capabilities": {},
        "workspaceFolders": workspace_folders(&[&manifest_root, &fallback_root])
    }));
    std::fs::write(&fallback, "oops\n:: standalone\n")
        .unwrap_or_else(|error| panic!("write malformed fallback source: {error}"));
    harness.notify(
        "textDocument/didSave",
        json!({ "textDocument": { "uri": fallback_uri.clone() } }),
    );
    let messages = harness.barrier(&fallback_uri);
    let published = published_diagnostics(&messages, &fallback_uri);
    assert!(
        !published["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("diagnostics array is missing"))
            .is_empty(),
        "the second workspace folder should retain a saved project document"
    );
    harness.finish();
}

#[test]
fn stdio_excluded_open_file_is_diagnosable_without_cross_project_diagnostics() {
    let temp = Builder::new()
        .prefix("recite % stdio excluded ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary workspace: {error}"));
    std::fs::write(
        temp.path().join("recite.project.toml"),
        "format_version = 1\n[discovery]\nsource_roots = [\"dialogue\"]\n",
    )
    .unwrap_or_else(|error| panic!("write project manifest: {error}"));
    std::fs::create_dir_all(temp.path().join("dialogue"))
        .unwrap_or_else(|error| panic!("create dialogue root: {error}"));
    std::fs::write(
        temp.path().join("dialogue/kept.recite"),
        ":: kept default\n> shared@83709c28414d0ce4659c\n  Kept.\n",
    )
    .unwrap_or_else(|error| panic!("write project source: {error}"));
    let excluded = temp.path().join("generated.recite");
    let excluded_uri = file_uri(&excluded);
    let mut harness = StdioHarness::start(json!({
        "capabilities": {},
        "workspaceFolders": workspace_folders(&[temp.path()])
    }));

    harness.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": excluded_uri.clone(),
                "languageId": "recite",
                "version": 1,
                "text": ":: generated default\n> shared@83709c28414d0ce4659c\n  Generated.\n"
            }
        }),
    );
    let messages = harness.barrier(&excluded_uri);
    let isolated = published_diagnostics(&messages, &excluded_uri);
    assert_eq!(
        isolated["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("diagnostics array is missing"))
            .len(),
        0,
        "an excluded buffer must not be merged with project documents: {isolated}"
    );

    harness.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": excluded_uri.clone(), "version": 2 },
            "contentChanges": [{ "text": "oops\n" }]
        }),
    );
    let messages = harness.barrier(&excluded_uri);
    let malformed = published_diagnostics(&messages, &excluded_uri);
    assert!(
        !malformed["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("diagnostics array is missing"))
            .is_empty(),
        "an excluded buffer still receives its own parser diagnostics"
    );
    harness.finish();
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

fn published_diagnostics(messages: &[Value], uri: &str) -> Value {
    let matches = messages
        .iter()
        .filter(|message| {
            message["method"] == "textDocument/publishDiagnostics"
                && message["params"]["uri"] == uri
        })
        .map(|message| message["params"].clone())
        .collect::<Vec<_>>();
    assert_eq!(
        matches.len(),
        1,
        "expected one diagnostics notification for {uri}, got {messages:?}"
    );
    matches
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("diagnostics notification disappeared"))
}
