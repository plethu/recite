use lsp_types::Uri;
use serde_json::json;
use tempfile::TempDir;

use super::super::super::support::{Harness, file_uri, write_file};
use crate::workspace::{DiagnosticRefresh, WorkspaceConfig};

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
    let startup_clear = harness.recv_publish_diagnostics();
    assert_eq!(startup_clear.uri, canonical_uri);
    assert_eq!(startup_clear.version, None);
    assert!(startup_clear.diagnostics.is_empty());
    let invalid = harness.recv_publish_diagnostics();
    assert_eq!(invalid.uri, alias_uri);
    assert_eq!(invalid.version, Some(7));
    assert!(!invalid.diagnostics.is_empty());

    harness.did_close(alias_uri.clone());
    let alias_refresh = harness.recv_publish_diagnostics();
    assert_eq!(alias_refresh.uri, alias_uri);
    assert_eq!(alias_refresh.version, None);
    assert!(alias_refresh.diagnostics.is_empty());
    let canonical_refresh = harness.recv_publish_diagnostics();
    assert_eq!(canonical_refresh.uri, canonical_uri);
    assert_eq!(canonical_refresh.version, None);
    assert!(canonical_refresh.diagnostics.is_empty());

    harness.finish();
}

pub(super) fn retired_schema_alias_close_clears_and_reopens() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n[project]\nschema = \"old.toml\"\n",
    );
    write_file(
        temp.path(),
        "old.toml",
        "schema_version = 1\n[producer]\nid = \"old\"\n",
    );
    write_file(
        temp.path(),
        "new.toml",
        "schema_version = 1\n[producer]\nid = \"new\"\n",
    );
    let params = json!({
        "capabilities": {},
        "rootUri": file_uri(temp.path()).as_str(),
    });
    let params =
        serde_json::from_value(params).unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = super::super::super::support::test_workspace(
        WorkspaceConfig::from_initialize_params(&params),
    );
    let manifest_uri = file_uri(&temp.path().join("recite.project.toml"));
    let alias_uri = format!("{}/./old.toml", file_uri(temp.path()).as_str())
        .parse::<Uri>()
        .unwrap_or_else(|error| panic!("alias URI: {error}"));

    workspace
        .open(alias_uri.clone(), 7, "not a schema\n".to_owned())
        .expect("active schema alias should publish diagnostics");
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n[project]\nschema = \"new.toml\"\n",
    );
    workspace.refresh_watched_uri(&manifest_uri);

    let close_refreshes = workspace.close(alias_uri.clone());
    assert!(close_refreshes.iter().any(|refresh| matches!(
        refresh,
        DiagnosticRefresh::Clear { uri, .. } if uri == &alias_uri
    )));
    let reopen = workspace
        .open(alias_uri.clone(), 8, ":: source default\n".to_owned())
        .expect("closed retired alias should be reopenable");
    let DiagnosticRefresh::Publish(reopened) = reopen else {
        panic!("reopening a retired alias should publish its exact URI");
    };
    assert_eq!(reopened.uri, alias_uri);
    assert_eq!(reopened.version, Some(8));
    assert!(
        reopened.diagnostics.is_empty(),
        "reopened diagnostics: {:?}",
        reopened.diagnostics
    );
}
