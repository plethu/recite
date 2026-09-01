mod support;

use serde_json::{Value, json};
use tempfile::Builder;

use support::stdio::{StdioHarness, file_uri};

#[test]
fn mapped_target_close_preserves_other_missing_schema_retirement() {
    let temp = Builder::new()
        .prefix("recite mapped target close ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary schema workspace: {error}"));
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    let first_manifest = first.join("recite.project.toml");
    let second_manifest = second.join("recite.project.toml");
    let first_old = first.join("schema-old.json");
    let first_new = first.join("schema-new.json");
    let second_old = second.join("schema-old.json");
    let second_new = second.join("schema-new.json");
    std::fs::create_dir_all(&first)
        .unwrap_or_else(|error| panic!("create first schema root: {error}"));
    std::fs::create_dir_all(second.join("sub"))
        .unwrap_or_else(|error| panic!("create missing-schema alias parent: {error}"));
    std::fs::write(
        &first_manifest,
        "format_version = 1\n[project]\nschema = \"schema-old.json\"\n",
    )
    .unwrap_or_else(|error| panic!("write first manifest: {error}"));
    std::fs::write(
        &second_manifest,
        "format_version = 1\n[project]\nschema = \"schema-old.json\"\n",
    )
    .unwrap_or_else(|error| panic!("write second manifest: {error}"));
    std::fs::write(&first_old, "{\"schema_version\":1}\n")
        .unwrap_or_else(|error| panic!("write mapped schema: {error}"));
    std::fs::write(&first_new, "{\"schema_version\":1}\n")
        .unwrap_or_else(|error| panic!("write mapped replacement: {error}"));
    std::fs::write(&second_new, "{\"schema_version\":1}\n")
        .unwrap_or_else(|error| panic!("write missing-schema replacement: {error}"));

    let first_uri = file_uri(&first_old);
    let second_uri = file_uri(&second_old);
    let second_alias_uri = format!("{}/sub/../schema-old.json", file_uri(&second));
    let mut harness = StdioHarness::start(json!({
        "capabilities": {},
        "workspaceFolders": [
            {"uri": file_uri(&first), "name": "first"},
            {"uri": file_uri(&second), "name": "second"}
        ]
    }));

    harness.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": first_uri.clone(), "languageId": "json", "version": 1, "text": "{\"schema_version\":\"overlay\"}\n"}}),
    );
    let _ = harness.barrier(&first_uri);
    harness.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": second_uri.clone(), "languageId": "json", "version": 1, "text": "oops\n"}}),
    );
    let _ = harness.barrier(&second_uri);

    std::fs::write(
        &first_manifest,
        "format_version = 1\n[project]\nschema = \"schema-new.json\"\n",
    )
    .unwrap_or_else(|error| panic!("switch mapped schema: {error}"));
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({"changes": [{"uri": file_uri(&first_manifest), "type": 2}]}),
    );
    let _ = harness.barrier(&first_uri);

    std::fs::write(
        &second_manifest,
        "format_version = 1\n[project]\nschema = \"schema-new.json\"\n",
    )
    .unwrap_or_else(|error| panic!("switch missing schema: {error}"));
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({"changes": [{"uri": file_uri(&second_manifest), "type": 2}]}),
    );
    let _ = harness.barrier(&second_uri);

    harness.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": first_uri.clone()}}),
    );
    let first_closed = harness.barrier(&first_uri);
    assert!(
        published_for(&first_closed, &first_uri)
            .iter()
            .any(|message| message["params"]["diagnostics"]
                .as_array()
                .is_some_and(Vec::is_empty))
    );

    // The late alias resolves to the missing second target after the mapped
    // first target has been fully closed. It must remain retired and source
    // diagnostics must stay suppressed.
    harness.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": second_alias_uri.clone(), "languageId": "json", "version": 2, "text": "oops\n"}}),
    );
    let opened_alias = harness.barrier(&second_alias_uri);
    assert!(
        published_for(&opened_alias, &second_alias_uri)
            .iter()
            .any(|message| message["params"]["diagnostics"]
                .as_array()
                .is_some_and(Vec::is_empty))
    );
    harness.notify(
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": second_alias_uri.clone(), "version": 3},
            "contentChanges": [{"text": "oops\n"}]
        }),
    );
    let changed_alias = harness.barrier(&second_alias_uri);
    assert!(
        published_for(&changed_alias, &second_alias_uri)
            .iter()
            .any(|message| message["params"]["diagnostics"]
                .as_array()
                .is_some_and(Vec::is_empty))
    );

    harness.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": second_uri.clone()}}),
    );
    let _ = harness.barrier(&second_uri);
    harness.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": second_alias_uri.clone()}}),
    );
    let _ = harness.barrier(&second_alias_uri);
    harness.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": second_alias_uri.clone(), "languageId": "json", "version": 4, "text": "oops\n"}}),
    );
    let reopened_alias = harness.barrier(&second_alias_uri);
    assert!(
        published_for(&reopened_alias, &second_alias_uri)
            .iter()
            .any(|message| message["params"]["diagnostics"]
                .as_array()
                .is_some_and(|diagnostics| !diagnostics.is_empty()))
    );
    harness.finish();
}

fn published_for<'a>(messages: &'a [Value], uri: &str) -> Vec<&'a Value> {
    messages
        .iter()
        .filter(|message| {
            message["method"] == "textDocument/publishDiagnostics"
                && message["params"]["uri"] == uri
        })
        .collect()
}
