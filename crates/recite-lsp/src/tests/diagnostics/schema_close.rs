use lsp_types::Uri;
use serde_json::json;
use tempfile::TempDir;

use super::super::super::support::{Harness, file_uri, write_file};

pub(super) fn did_close_schema_alias_clears_exact_uri() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let schema = "schema_version = 1\n[producer]\nid = \"dialogue\"\n";
    let schema_path = temp.path().join("standalone.toml");
    write_file(temp.path(), "standalone.toml", schema);
    let harness = Harness::start_with_result(json!({
        "capabilities": {},
        "rootUri": file_uri(temp.path()).as_str(),
        "initializationOptions": {
            "schema": schema_path.display().to_string()
        }
    }))
    .0;
    let alias_uri = format!("{}/./standalone.toml", file_uri(temp.path()).as_str())
        .parse::<Uri>()
        .unwrap_or_else(|error| panic!("alias URI: {error}"));
    let canonical_uri = file_uri(&schema_path);

    harness.did_open(alias_uri.clone(), 7, "not a schema\n");
    let invalid = harness.recv_publish_diagnostics();
    assert_eq!(invalid.uri, canonical_uri);
    assert_eq!(invalid.version, Some(7));
    assert!(!invalid.diagnostics.is_empty());

    harness.did_close(alias_uri.clone());
    let canonical_refresh = harness.recv_publish_diagnostics();
    assert_eq!(canonical_refresh.uri, canonical_uri);
    assert_eq!(canonical_refresh.version, None);
    assert!(canonical_refresh.diagnostics.is_empty());

    harness.finish();
}
