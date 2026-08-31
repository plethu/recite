#[path = "support/stdio.rs"]
mod stdio;

use serde_json::json;
use stdio::{StdioHarness, file_uri};
use tempfile::Builder;

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
    let manifest_uri = file_uri(&manifest);
    let mut harness = StdioHarness::start_with_schema_option(temp.path(), None);

    harness.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": schema_a_uri.clone(),
                "languageId": "json",
                "version": 7,
                "text": "{\"schema_version\":\"overlay\"}\n"
            }
        }),
    );
    let overlay = harness.expect_diagnostics(&schema_a_uri);
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
    let refreshed_overlay = harness.expect_diagnostics(&schema_a_uri);
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
    let old_clear = harness.expect_diagnostics(&schema_a_uri);
    assert!(
        old_clear["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("old schema diagnostics array is missing"))
            .is_empty()
    );
    let replacement = harness.expect_diagnostics(&schema_b_uri);
    assert!(
        replacement["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("replacement diagnostics array is missing"))
            .iter()
            .any(|diagnostic| diagnostic["message"]
                .as_str()
                .is_some_and(|message| message.contains("schema_version must be an integer")))
    );

    harness.finish();
}
