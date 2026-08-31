use lsp_types::NumberOrString;
use serde_json::json;
use tempfile::TempDir;

use crate::workspace::WorkspaceConfig;

use super::super::support::{Harness, block_names, file_uri, test_workspace, write_file};

pub(crate) fn identical_relative_keys_are_partitioned() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    for (root, name) in [(&first, "first"), (&second, "second")] {
        write_file(
            root,
            "recite.project.toml",
            "format_version = 1\n[discovery]\nsource_roots = [\"src\"]\n",
        );
        write_file(
            root,
            "src/main.recite",
            &format!(":: start default\n:: {name}\n"),
        );
    }
    let params = serde_json::from_value(json!({
        "workspaceFolders": [
            {"uri": file_uri(&first).as_str(), "name": "first"},
            {"uri": file_uri(&second).as_str(), "name": "second"}
        ],
        "capabilities": {}
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));
    let summaries = workspace.snapshot().summaries();
    assert_eq!(summaries.len(), 2);
    assert_eq!(
        summaries
            .iter()
            .filter_map(|summary| summary.project_relative_path())
            .collect::<Vec<_>>(),
        ["src/main.recite", "src/main.recite"]
    );
    assert_eq!(
        block_names(&workspace),
        ["start", "first", "start", "second"]
    );
}

pub(crate) fn identical_relative_keys_use_their_project_schema() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    let schema = |value: &str| {
        format!(
            r#"{{
  "schema_version": 1,
  "registries": {{"sound": {{"values": ["{value}"]}}}},
  "effects": {{"play_sfx": {{"modes": ["immediate"], "params": [{{"name": "sound_effect", "type": "registry:sound"}}]}}}}
}}"#
        )
    };
    for (root, value) in [(&first, "first"), (&second, "second")] {
        write_file(
            root,
            "recite.project.toml",
            "format_version = 1\n[project]\nschema = \"schema.json\"\n[discovery]\nsource_roots = [\"src\"]\n",
        );
        write_file(root, "schema.json", &schema(value));
        write_file(root, "src/main.recite", ":: start default\n");
    }
    let params = serde_json::from_value(json!({
        "workspaceFolders": [
            {"uri": file_uri(&first).as_str(), "name": "first"},
            {"uri": file_uri(&second).as_str(), "name": "second"}
        ],
        "capabilities": {}
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let (harness, _) = Harness::start_with_result(params);
    let first_uri = file_uri(&first.join("src/main.recite"));
    let second_uri = file_uri(&second.join("src/main.recite"));
    let source = ":: start default\n! immediate play_sfx(first)\n";

    harness.did_open(first_uri.clone(), 1, source);
    let first_diagnostics = harness.recv_publish_diagnostics();
    assert_eq!(first_diagnostics.uri, first_uri);
    assert!(first_diagnostics.diagnostics.is_empty());

    harness.did_open(second_uri.clone(), 1, source);
    let second_diagnostics = harness.recv_publish_diagnostics();
    assert_eq!(second_diagnostics.uri, second_uri);
    assert_eq!(second_diagnostics.diagnostics.len(), 1);
    assert_eq!(
        second_diagnostics.diagnostics[0].code,
        Some(NumberOrString::String("RECITE_VALIDATE021".to_owned()))
    );
    let refreshed_first = harness.recv_publish_diagnostics();
    assert_eq!(refreshed_first.uri, first_uri);
    assert!(refreshed_first.diagnostics.is_empty());
    harness.finish();
}
