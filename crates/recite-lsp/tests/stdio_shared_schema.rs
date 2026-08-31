mod support;

use serde_json::{Value, json};
use tempfile::Builder;

use support::stdio::{StdioHarness, file_uri};

#[test]
fn shared_schema_owner_transitions_coalesce_and_reactivate() {
    let temp = Builder::new()
        .prefix("recite % stdio shared schema ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary workspace: {error}"));
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    let shared_schema = temp.path().join("shared.json");
    write_file(
        &shared_schema,
        r#"{"schema_version":"invalid"}
"#,
    );
    for root in [&first, &second] {
        write_file(
            &root.join("recite.project.toml"),
            "format_version = 1\n[project]\nschema = \"../shared.json\"\n",
        );
    }
    let schema_uri = format!("{}/../shared.json", file_uri(&first));
    let first_manifest_path = first.join("recite.project.toml");
    let second_manifest_path = second.join("recite.project.toml");
    let first_manifest = file_uri(&first_manifest_path);
    let second_manifest = file_uri(&second_manifest_path);
    let mut harness = StdioHarness::start(json!({
        "capabilities": {},
        "workspaceFolders": [
            {"uri": file_uri(&first), "name": "first"},
            {"uri": file_uri(&second), "name": "second"}
        ]
    }));
    assert_schema_batch(&harness.barrier(&schema_uri), &schema_uri, None, false);

    harness.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": schema_uri.clone(),
                "languageId": "json",
                "version": 3,
                "text": "{\"schema_version\":\"overlay\"}\n"
            }
        }),
    );
    assert_schema_batch(&harness.barrier(&schema_uri), &schema_uri, Some(3), false);

    std::fs::remove_file(&first_manifest_path)
        .unwrap_or_else(|error| panic!("remove first manifest: {error}"));
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": first_manifest, "type": 3 }] }),
    );
    assert_schema_batch(&harness.barrier(&schema_uri), &schema_uri, Some(3), false);

    std::fs::remove_file(&second_manifest_path)
        .unwrap_or_else(|error| panic!("remove second manifest: {error}"));
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": second_manifest, "type": 3 }] }),
    );
    assert_schema_batch(&harness.barrier(&schema_uri), &schema_uri, Some(3), true);

    write_file(
        &first.join("recite.project.toml"),
        "format_version = 1\n[project]\nschema = \"../shared.json\"\n",
    );
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": file_uri(&first.join("recite.project.toml")), "type": 1 }] }),
    );
    assert_schema_batch(&harness.barrier(&schema_uri), &schema_uri, Some(3), false);
    harness.finish();
}

fn assert_schema_batch(messages: &[Value], uri: &str, version: Option<i64>, clear: bool) {
    assert_eq!(messages.len(), 1, "schema lifecycle batch: {messages:?}");
    let message = &messages[0];
    assert_eq!(message["method"], "textDocument/publishDiagnostics");
    assert_eq!(message["params"]["uri"], uri);
    assert_eq!(message["params"]["version"].as_i64(), version);
    assert_eq!(
        message["params"]["diagnostics"]
            .as_array()
            .is_some_and(Vec::is_empty),
        clear,
        "schema lifecycle payload: {message}"
    );
}

fn write_file(path: &std::path::Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("create parent directory: {error}"));
    }
    std::fs::write(path, text).unwrap_or_else(|error| panic!("write test file: {error}"));
}
