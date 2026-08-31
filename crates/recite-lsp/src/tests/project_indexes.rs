use lsp_types::NumberOrString;
use recite_ui::DEFAULT_RESOURCE;
use serde_json::json;
use tempfile::TempDir;

use crate::workspace::{DiagnosticRefresh, WorkspaceChangeResult, WorkspaceConfig};

use super::support::{Harness, block_names, file_uri, full_change, test_workspace, write_file};

pub(super) mod discovery_transitions;
mod lifecycle;
mod schema_summary;
mod transactions;

pub(super) use lifecycle::{
    did_close_refreshes_saved_summary_before_falling_back,
    did_save_refreshes_saved_summary_for_closed_files,
    did_save_rekeys_new_open_file_without_duplicate_summary,
    manifest_refresh_is_atomic_and_preserves_open_overlay,
    open_alias_owner_switch_reseeds_kernel_version_state,
    open_nonexistent_aliases_share_one_fallback_key, open_summary_overlays_saved_project_summary,
    saved_uri_replacement_removes_old_canonical_entry,
    watched_files_refresh_saved_index_for_create_and_delete,
    watched_refresh_publishes_effective_open_payload,
};
pub(super) use transactions::{
    duplicate_open_is_ignored_transactionally, manifest_refresh_rekeys_open_overlay,
    watched_creation_rekeys_open_overlay,
};

pub(super) fn saved_project_discovery_is_deterministically_sorted() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "z.recite", ":: z\n");
    write_file(temp.path(), "a.recite", ":: a\n");
    write_file(temp.path(), "nested/m.recite", ":: nested\n");
    write_file(temp.path(), "target/ignored.recite", ":: ignored\n");
    write_file(temp.path(), ".hidden/ignored.recite", ":: ignored\n");

    let workspace = test_workspace(WorkspaceConfig::for_roots(vec![temp.path().to_owned()]));
    let paths = workspace
        .snapshot()
        .summaries()
        .iter()
        .map(|summary| {
            summary
                .project_relative_path()
                .unwrap_or("<none>")
                .to_owned()
        })
        .collect::<Vec<_>>();

    assert_eq!(paths, ["a.recite", "nested/m.recite", "z.recite"]);
}

pub(super) fn manifest_discovery_uses_shared_source_roots() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n\n[discovery]\nsource_roots = [\"src\"]\n",
    );
    write_file(temp.path(), "src/kept.recite", ":: kept\n");
    write_file(temp.path(), "ignored.recite", ":: ignored\n");

    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));
    let paths = workspace
        .snapshot()
        .summaries()
        .iter()
        .map(|summary| {
            summary
                .project_relative_path()
                .unwrap_or("<none>")
                .to_owned()
        })
        .collect::<Vec<_>>();

    assert_eq!(paths, ["src/kept.recite"]);
}

pub(super) fn explicit_relative_schema_uses_project_root_with_multiple_source_roots() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n\n[discovery]\nsource_roots = [\"dialogue\", \"lore\"]\n",
    );
    write_file(temp.path(), "dialogue/scene.recite", ":: scene\n");
    write_file(temp.path(), "lore/notes.recite", ":: notes\n");
    write_file(temp.path(), "schema.json", "{\"schema_version\":1}\n");

    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
        "initializationOptions": { "schema": "schema.json" }
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));

    let paths = workspace
        .snapshot()
        .summaries()
        .iter()
        .filter_map(|summary| summary.project_relative_path())
        .collect::<Vec<_>>();
    assert_eq!(paths, ["dialogue/scene.recite", "lore/notes.recite"]);
    assert!(workspace.schema().summary().is_some());
}

pub(super) fn explicit_schema_override_survives_manifest_schema_change() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n[project]\nschema = \"manifest.json\"\n",
    );
    write_file(temp.path(), "manifest.json", "{\"schema_version\":1}\n");
    write_file(temp.path(), "override.json", "{\"schema_version\":1}\n");

    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
        "initializationOptions": { "schema": "override.json" }
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));
    let manifest_uri = file_uri(&temp.path().join("recite.project.toml"));

    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n[project]\nschema = \"replacement.json\"\n",
    );
    write_file(temp.path(), "replacement.json", "{\"schema_version\":2}\n");
    workspace.refresh_watched_uri(&manifest_uri);

    assert!(workspace.schema().summary().is_some());
    assert!(workspace.schema_diagnostics().is_none());
}

pub(super) fn malformed_manifest_does_not_fall_back_to_saved_walker() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let manifest = temp.path().join("recite.project.toml");
    write_file(temp.path(), "recite.project.toml", "format_version = [\n");
    write_file(temp.path(), "source.recite", ":: saved\n");

    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));

    assert!(workspace.snapshot().summaries().is_empty());
    let diagnostics = workspace
        .project_diagnostics()
        .expect("manifest diagnostics");
    let DiagnosticRefresh::Publish(diagnostics) = diagnostics else {
        panic!("expected manifest diagnostics")
    };
    assert_eq!(diagnostics.uri, file_uri(&manifest));
    assert_eq!(
        diagnostics.diagnostics[0].code.as_str(),
        "RECITE_PROJECT001"
    );
}

pub(super) fn schema_load_failure_keeps_source_only_snapshot() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "scene.recite", ":: source\n");
    write_file(temp.path(), "schema.json", r#"{"schema_version":"one"}"#);

    let workspace = test_workspace(
        WorkspaceConfig::for_roots(vec![temp.path().to_owned()])
            .with_schema_path(temp.path().join("schema.json")),
    );

    assert!(workspace.schema().summary().is_none());
    assert!(workspace.schema_diagnostics().is_some());
    assert_eq!(block_names(&workspace), ["source"]);
}

pub(super) fn initialized_publishes_schema_load_diagnostics() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let schema_path = temp.path().join("schema.json");
    let root_uri = file_uri(temp.path());
    let schema_uri = file_uri(&schema_path);
    let localized_resource = DEFAULT_RESOURCE.replace(
        "diagnostic-schema-001-read = failed to read schema manifest: {$detail}",
        "diagnostic-schema-001-read = localized schema read: {$detail}",
    );
    let (harness, _) = Harness::start_with_result_and_resource(
        json!({
        "capabilities": {},
        "rootUri": root_uri.as_str(),
        "initializationOptions": {
            "schema": schema_path.display().to_string()
        }
        }),
        "fr",
        localized_resource,
    );

    let published = harness.recv_publish_diagnostics();

    assert_eq!(published.uri, schema_uri);
    assert_eq!(published.version, None);
    assert_eq!(published.diagnostics.len(), 1);
    assert_eq!(
        published.diagnostics[0].code,
        Some(NumberOrString::String("RECITE_SCHEMA001".to_owned()))
    );
    assert!(
        published.diagnostics[0]
            .message
            .starts_with("localized schema read: ")
    );

    harness.finish();
}

pub(super) fn schema_projection_diagnostics_publish_and_clear_after_save() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let schema_path = temp.path().join("schema.json");
    write_file(temp.path(), "schema.json", invalid_projection_schema());
    let root_uri = file_uri(temp.path());
    let schema_uri = file_uri(&schema_path);
    let harness = Harness::start_with_result(json!({
        "capabilities": {
            "general": {
                "positionEncodings": ["utf-16"]
            }
        },
        "rootUri": root_uri.as_str(),
        "initializationOptions": {
            "schema": schema_path.display().to_string()
        }
    }))
    .0;

    let published = harness.recv_publish_diagnostics();
    assert_eq!(published.uri, schema_uri);
    assert!(published.diagnostics.iter().any(|diagnostic| {
        diagnostic.code.as_ref() == Some(&NumberOrString::String("RECITE_SCHEMA004".to_owned()))
            && diagnostic
                .message
                .contains("unknown projection query function 'missing'")
    }));

    write_file(
        temp.path(),
        "schema.json",
        schema_summary::valid_projection_schema(),
    );
    harness.did_save(schema_uri);
    let cleared = harness.recv_publish_diagnostics();
    assert!(cleared.diagnostics.is_empty());

    harness.finish();
}

pub(super) fn metadata_domain_schema_summary_preserves_available_provenance() {
    schema_summary::metadata_domain_schema_summary_preserves_available_provenance();
}

pub(super) fn projection_schema_summary_exposes_queries_projectors_and_labels() {
    schema_summary::projection_schema_summary_exposes_queries_projectors_and_labels();
}

pub(super) fn schema_summary_preserves_source_ownership_and_generated_read_only_state() {
    schema_summary::schema_summary_preserves_source_ownership_and_generated_read_only_state();
}

#[cfg(unix)]
pub(super) fn schema_kind_survives_symlink_reload() {
    schema_summary::schema_kind_survives_symlink_reload();
}

fn invalid_projection_schema() -> &'static str {
    r#"{
  "schema_version": 1,
  "metadata": {
    "skill": { "targets": ["choice"], "type": "string" }
  },
  "presentation_projectors": {
    "choice_skill_prefix": {
      "candidates": { "kind": "metadata_key", "target": "choice", "key": "skill" },
      "queries": {
        "current": { "function": "missing", "args": [] }
      }
    }
  }
}"#
}

pub(super) fn stale_change_does_not_bump_snapshot_generation() {
    let mut workspace = test_workspace(WorkspaceConfig::for_roots(Vec::new()));
    let uri = super::support::uri("file:///workspace/dialogue/stale-generation.recite");
    workspace.open(uri.clone(), 3, ":: live\n".to_owned());
    let generation = workspace.generation();

    match workspace.change(uri, 2, vec![full_change("oops\n:: stale\n")]) {
        WorkspaceChangeResult::Stale => {}
        other => panic!("expected stale change, got {other:?}"),
    }

    assert_eq!(workspace.generation(), generation);
    assert_eq!(workspace.snapshot().generation(), generation);
}
