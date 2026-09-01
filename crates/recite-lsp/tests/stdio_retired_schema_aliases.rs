mod support;

use serde_json::{Value, json};
use tempfile::Builder;

use support::stdio::{StdioHarness, file_uri};

#[test]
fn retired_schema_aliases_keep_target_retirement_until_final_close() {
    let temp = Builder::new()
        .prefix("recite retired schema aliases ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary schema directory: {error}"));
    let manifest = temp.path().join("recite.project.toml");
    let old_schema = temp.path().join("schema-old.json");
    let new_schema = temp.path().join("schema-new.json");
    std::fs::write(
        &manifest,
        "format_version = 1\n[project]\nschema = \"schema-old.json\"\n",
    )
    .unwrap_or_else(|error| panic!("write manifest: {error}"));
    std::fs::write(&old_schema, "{\"schema_version\":1}\n")
        .unwrap_or_else(|error| panic!("write old schema: {error}"));
    std::fs::write(&new_schema, "{\"schema_version\":\"new\"}\n")
        .unwrap_or_else(|error| panic!("write new schema: {error}"));

    let schema_a = format!("{}/./schema-old.json", file_uri(temp.path()));
    let schema_b = format!("{}/sub/../schema-old.json", file_uri(temp.path()));
    let new_schema_uri = file_uri(&new_schema);
    let manifest_uri = file_uri(&manifest);
    let mut harness = StdioHarness::start(json!({
        "capabilities": {},
        "rootUri": file_uri(temp.path())
    }));

    for (uri, version) in [(&schema_a, 7_i64), (&schema_b, 8_i64)] {
        harness.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "json",
                    "version": version,
                    "text": "{\"schema_version\":\"overlay\"}\n"
                }
            }),
        );
    }
    let opened_a = harness.barrier(&schema_a);
    assert_schema_publish(&opened_a, &file_uri(&old_schema), Some(7));
    let opened_b = harness.barrier(&schema_b);
    assert!(
        opened_b.is_empty(),
        "second alias should not publish stale diagnostics: {opened_b:?}"
    );

    std::fs::write(
        &manifest,
        "format_version = 1\n[project]\nschema = \"schema-new.json\"\n",
    )
    .unwrap_or_else(|error| panic!("switch schema target: {error}"));
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": manifest_uri, "type": 2 }] }),
    );
    let switched = harness.barrier(&schema_a);
    assert_eq!(switched.len(), 2, "schema switch batch: {switched:?}");
    assert_schema_clear(&switched, &schema_a, Some(7));
    assert_schema_publish(&switched, &new_schema_uri, None);
    let switched_b = harness.barrier(&schema_b);
    assert!(
        switched_b.is_empty(),
        "second alias should not receive a stale switch batch: {switched_b:?}"
    );

    harness.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": schema_a.clone() } }),
    );
    let closed_a = harness.barrier(&schema_a);
    assert_eq!(closed_a.len(), 2, "close A batch: {closed_a:?}");
    assert_schema_clear(&closed_a, &schema_a, None);
    assert_schema_publish(&closed_a, &schema_b, Some(8));

    harness.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": schema_a.clone(),
                "languageId": "json",
                "version": 9,
                "text": "{\"schema_version\":\"reopened\"}\n"
            }
        }),
    );
    let reopened_a = harness.barrier(&schema_a);
    assert_eq!(reopened_a.len(), 1, "reopen A batch: {reopened_a:?}");
    assert_schema_publish(&reopened_a, &schema_a, Some(9));
    assert_empty_diagnostics(&reopened_a, &schema_a);

    harness.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": schema_b.clone() } }),
    );
    let closed_b = harness.barrier(&schema_b);
    assert_eq!(closed_b.len(), 2, "close B final-owner batch: {closed_b:?}");
    assert_schema_clear(&closed_b, &schema_b, None);
    assert_schema_publish(&closed_b, &schema_a, Some(9));

    harness.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": schema_a.clone() } }),
    );
    let closed_final = harness.barrier(&schema_a);
    assert_eq!(closed_final.len(), 1, "final close batch: {closed_final:?}");
    assert_schema_clear(&closed_final, &schema_a, None);
    harness.finish();
}

fn assert_schema_publish(messages: &[Value], uri: &str, version: Option<i64>) {
    let published = diagnostics_for(messages, uri);
    assert_eq!(published.len(), 1, "publish {uri}: {messages:?}");
    assert_eq!(published[0]["params"]["version"].as_i64(), version);
}

fn assert_schema_clear(messages: &[Value], uri: &str, version: Option<i64>) {
    let published = diagnostics_for(messages, uri);
    assert_eq!(published.len(), 1, "clear {uri}: {messages:?}");
    assert_eq!(published[0]["params"]["version"].as_i64(), version);
    assert_empty_diagnostics(messages, uri);
}

fn assert_empty_diagnostics(messages: &[Value], uri: &str) {
    let published = diagnostics_for(messages, uri);
    assert!(
        published[0]["params"]["diagnostics"]
            .as_array()
            .is_some_and(Vec::is_empty)
    );
}

fn diagnostics_for<'a>(messages: &'a [Value], uri: &str) -> Vec<&'a Value> {
    messages
        .iter()
        .filter(|message| message["method"] == "textDocument/publishDiagnostics")
        .filter(|message| message["params"]["uri"] == uri)
        .collect()
}
