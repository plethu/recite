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
    let published = published_diagnostics(&messages, &fallback_uri, None);
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
fn stdio_malformed_workspace_folder_does_not_block_later_folder() {
    let temp = Builder::new()
        .prefix("recite % stdio malformed workspace ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary workspace: {error}"));
    let malformed_root = temp.path().join("malformed");
    let valid_root = temp.path().join("valid");
    std::fs::create_dir_all(&malformed_root)
        .unwrap_or_else(|error| panic!("create malformed root: {error}"));
    std::fs::write(
        malformed_root.join("recite.project.toml"),
        "format_version = [\n",
    )
    .unwrap_or_else(|error| panic!("write malformed manifest: {error}"));
    std::fs::write(malformed_root.join("leaked.recite"), ":: leaked\n")
        .unwrap_or_else(|error| panic!("write malformed source: {error}"));
    std::fs::create_dir_all(&valid_root)
        .unwrap_or_else(|error| panic!("create valid root: {error}"));
    std::fs::write(
        valid_root.join("recite.project.toml"),
        "format_version = 1\n[discovery]\nsource_roots = [\"src\"]\n",
    )
    .unwrap_or_else(|error| panic!("write valid manifest: {error}"));
    std::fs::create_dir_all(valid_root.join("src"))
        .unwrap_or_else(|error| panic!("create valid source root: {error}"));
    let valid = valid_root.join("src/references.recite");
    std::fs::write(
        &valid,
        ":: start default\n> intro@8a535b2e538dd4f39758\n  Hello.\n-> src/definitions.recite::target\n",
    )
        .unwrap_or_else(|error| panic!("write valid source: {error}"));
    let definition = valid_root.join("src/definitions.recite");
    std::fs::write(
        &definition,
        ":: target\n> line@b7cf36a63a75edb16a8f\n  There.\n",
    )
    .unwrap_or_else(|error| panic!("write definition source: {error}"));

    let malformed_manifest_uri = file_uri(&malformed_root.join("recite.project.toml"));
    let mut harness = StdioHarness::start(json!({
        "capabilities": {},
        "workspaceFolders": workspace_folders(&[&malformed_root, &valid_root])
    }));
    let startup = harness.receive_message();
    assert_eq!(startup["method"], "textDocument/publishDiagnostics");
    assert_eq!(startup["params"]["uri"], malformed_manifest_uri);
    assert_eq!(startup["params"]["version"], Value::Null);
    assert_eq!(
        startup["params"]["diagnostics"][0]["code"],
        "RECITE_PROJECT001"
    );

    let valid_uri = file_uri(&valid);
    let definition_uri = file_uri(&definition);
    harness.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": valid_uri,
                "languageId": "recite",
                "version": 1,
                "text": ":: start default\n> intro@8a535b2e538dd4f39758\n  Hello.\n-> src/definitions.recite::target\n"
            }
        }),
    );
    let messages = harness.barrier(&valid_uri);
    let diagnostics = published_diagnostics(&messages, &valid_uri, Some(1));
    assert!(diagnostics["diagnostics"].as_array().unwrap().is_empty());
    let definition_id = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": valid_uri },
            "position": { "line": 3, "character": 29 }
        }),
    );
    let definition_response = harness.receive_message();
    assert_eq!(definition_response["id"], json!(definition_id));
    assert_eq!(
        definition_response["result"]["uri"], definition_uri,
        "valid workspace must retain cross-file project identity"
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
    let isolated = published_diagnostics(&messages, &excluded_uri, Some(1));
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
    let malformed = published_diagnostics(&messages, &excluded_uri, Some(2));
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

fn published_diagnostics(messages: &[Value], uri: &str, version: Option<i32>) -> Value {
    assert_eq!(
        messages.len(),
        1,
        "expected one notification for {uri}, got {messages:?}"
    );
    let message = messages
        .first()
        .unwrap_or_else(|| panic!("notification disappeared"));
    assert_eq!(
        message["method"], "textDocument/publishDiagnostics",
        "unexpected notification for {uri}: {message}"
    );
    let params = &message["params"];
    assert_eq!(
        params["uri"], uri,
        "diagnostics notification has the wrong URI: {message}"
    );
    assert_eq!(
        params["version"],
        version.map_or(Value::Null, Value::from),
        "diagnostics notification has the wrong version: {message}"
    );
    assert!(
        params["diagnostics"].is_array(),
        "diagnostics notification has no diagnostics array: {message}"
    );
    params.clone()
}
