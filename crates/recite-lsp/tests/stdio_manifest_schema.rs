#[path = "support/stdio.rs"]
mod stdio;

use serde_json::json;
use stdio::{StdioHarness, file_uri};
use tempfile::Builder;
use url::Url;

#[test]
fn manifest_schema_change_reloads_and_preserves_open_overlay() {
    let temp = Builder::new()
        .prefix("recite manifest schema stdio ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary schema directory: {error}"));
    let manifest = temp.path().join("recite.project.toml");
    let schema_a = temp.path().join("schema-a.json");
    let schema_b = temp.path().join("schema-b.json");
    std::fs::write(
        &manifest,
        "format_version = 1\n[project]\nschema = \"schema-a.json\"\n",
    )
    .unwrap_or_else(|error| panic!("write initial manifest: {error}"));
    std::fs::write(&schema_a, "{\"schema_version\":1}\n")
        .unwrap_or_else(|error| panic!("write initial schema: {error}"));
    std::fs::write(&schema_b, "{\"schema_version\":\"new\"}\n")
        .unwrap_or_else(|error| panic!("write replacement schema: {error}"));

    let schema_a_uri = file_uri(&schema_a);
    let schema_b_uri = file_uri(&schema_b);
    let schema_a_alias_uri = Url::from_file_path(temp.path())
        .unwrap_or_else(|()| panic!("temporary directory cannot become a file URI"));
    let schema_a_alias_uri = format!("{schema_a_alias_uri}/./schema-a.json");
    assert_ne!(schema_a_uri, schema_a_alias_uri);
    let manifest_uri = file_uri(&manifest);
    let mut harness = StdioHarness::start(json!({
        "capabilities": {},
        "rootUri": file_uri(temp.path())
    }));

    harness.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": schema_a_alias_uri.clone(),
                "languageId": "json",
                "version": 7,
                "text": "{\"schema_version\":\"overlay\"}\n"
            }
        }),
    );
    let overlay = harness.expect_diagnostics(&schema_a_alias_uri);
    assert_eq!(overlay["version"], 7);
    assert!(
        !overlay["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("overlay diagnostics array is missing"))
            .is_empty()
    );

    std::fs::write(
        &manifest,
        "format_version = 1\n[project]\nschema = \"schema-a.json\"\nversion = \"changed\"\n",
    )
    .unwrap_or_else(|error| panic!("write unchanged schema manifest: {error}"));
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": manifest_uri.clone(), "type": 2 }] }),
    );
    let refresh_messages = harness.barrier(&schema_a_alias_uri);
    let refreshed_diagnostics = diagnostics_for(&refresh_messages, &schema_a_alias_uri);
    assert_eq!(refreshed_diagnostics.len(), 1);
    let refreshed_overlay = refreshed_diagnostics[0];
    assert_eq!(refreshed_overlay["version"], 7);
    assert!(
        !refreshed_overlay["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("refreshed diagnostics array is missing"))
            .is_empty()
    );

    std::fs::write(
        &manifest,
        "format_version = 1\n[project]\nschema = \"schema-b.json\"\n",
    )
    .unwrap_or_else(|error| panic!("write replacement schema manifest: {error}"));
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": manifest_uri, "type": 2 }] }),
    );
    let switch_messages = harness.barrier(&schema_a_alias_uri);
    assert_eq!(switch_messages.len(), 2);
    let old_clear = diagnostics_for(&switch_messages, &schema_a_alias_uri);
    assert_eq!(old_clear.len(), 1);
    let old_clear = old_clear[0];
    assert_eq!(old_clear["version"], 7);
    assert!(
        old_clear["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("old schema diagnostics array is missing"))
            .is_empty()
    );
    let replacement = diagnostics_for(&switch_messages, &schema_b_uri);
    assert_eq!(replacement.len(), 1);
    let replacement = replacement[0];
    assert!(
        replacement["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("replacement diagnostics array is missing"))
            .iter()
            .any(|diagnostic| diagnostic["message"]
                .as_str()
                .is_some_and(|message| message.contains("schema_version must be an integer")))
    );

    harness.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": schema_a_alias_uri.clone(), "version": 8 },
            "contentChanges": [{ "text": "dialogue text\n" }]
        }),
    );
    let retired_change_messages = harness.barrier(&schema_a_alias_uri);
    assert_eq!(retired_change_messages.len(), 1);
    let retired_change = diagnostics_for(&retired_change_messages, &schema_a_alias_uri);
    assert_eq!(retired_change.len(), 1);
    assert_eq!(retired_change[0]["version"], 8);
    assert!(
        retired_change[0]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    std::fs::write(
        &manifest,
        "format_version = 1\n[project]\ncontent_set = \"removed-schema\"\n",
    )
    .unwrap_or_else(|error| panic!("remove manifest schema: {error}"));
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": file_uri(&manifest), "type": 2 }] }),
    );
    let removal_messages = harness.barrier(&schema_a_alias_uri);
    assert_eq!(removal_messages.len(), 1);
    let replacement_clear = diagnostics_for(&removal_messages, &schema_b_uri);
    assert_eq!(replacement_clear.len(), 1);
    assert!(replacement_clear[0]["version"].is_null());
    assert!(
        replacement_clear[0]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(diagnostics_for(&removal_messages, &schema_a_alias_uri).is_empty());

    std::fs::write(
        &manifest,
        "format_version = 1\n[project]\nschema = \"schema-a.json\"\n",
    )
    .unwrap_or_else(|error| panic!("re-add manifest schema: {error}"));
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": file_uri(&manifest), "type": 2 }] }),
    );
    let readd_messages = harness.barrier(&schema_a_alias_uri);
    assert_eq!(readd_messages.len(), 1);
    let readded = diagnostics_for(&readd_messages, &schema_a_alias_uri);
    assert_eq!(readded.len(), 1);
    assert_eq!(readded[0]["version"], 8);
    assert!(!readded[0]["diagnostics"].as_array().unwrap().is_empty());

    harness.finish();
}

fn diagnostics_for<'a>(messages: &'a [serde_json::Value], uri: &str) -> Vec<&'a serde_json::Value> {
    messages
        .iter()
        .filter(|message| message["method"] == "textDocument/publishDiagnostics")
        .filter(|message| message["params"]["uri"] == uri)
        .map(|message| &message["params"])
        .collect()
}
