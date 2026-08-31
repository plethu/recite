use serde_json::json;
use tempfile::Builder;
use url::Url;

#[path = "support/stdio.rs"]
mod stdio;

use stdio::{StdioHarness, file_uri};

#[test]
fn stdio_schema_alias_close_clears_alias_and_refreshes_canonical() {
    let temp = Builder::new()
        .prefix("recite % stdio ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary schema directory: {error}"));
    let schema = temp.path().join("standalone.toml");
    std::fs::write(
        &schema,
        "schema_version = 1\n[producer]\nid = \"dialogue\"\n",
    )
    .unwrap_or_else(|error| panic!("write standalone schema: {error}"));
    let canonical_uri = file_uri(&schema);
    let alias_base = Url::from_file_path(temp.path())
        .unwrap_or_else(|()| panic!("temporary directory cannot become a file URI"));
    let alias_uri = format!("{}/./standalone.toml", alias_base);
    assert_ne!(alias_uri, canonical_uri);
    assert!(alias_uri.contains("/./"));
    let schema_path = schema.display().to_string();
    let mut harness = StdioHarness::start_with_schema_option(temp.path(), Some(&schema_path));

    harness.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": alias_uri.clone(),
                "languageId": "toml",
                "version": 7,
                "text": "not a schema\n"
            }
        }),
    );
    let invalid = harness.expect_diagnostics(&alias_uri);
    assert_eq!(invalid["version"], 7);
    assert!(
        !invalid["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("diagnostics array is missing"))
            .is_empty()
    );

    harness.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": alias_uri.clone() } }),
    );
    let alias_clear = harness.expect_diagnostics(&alias_uri);
    assert!(alias_clear["version"].is_null());
    assert!(
        alias_clear["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("diagnostics array is missing"))
            .is_empty()
    );
    let canonical_refresh = harness.expect_diagnostics(&canonical_uri);
    assert!(canonical_refresh["version"].is_null());
    assert!(
        canonical_refresh["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("diagnostics array is missing"))
            .is_empty()
    );

    harness.finish();
}
