use lsp_types::NumberOrString;
use recite_ui::DEFAULT_RESOURCE;
use serde_json::json;
use tempfile::TempDir;

use crate::workspace::{LspWorkspace, WorkspaceChangeResult, WorkspaceConfig};

use super::support::{Harness, block_names, file_uri, full_change, harness_for_root, write_file};

mod schema_summary;

pub(super) fn saved_project_discovery_is_deterministically_sorted() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "z.recite", ":: z\n");
    write_file(temp.path(), "a.recite", ":: a\n");
    write_file(temp.path(), "nested/m.recite", ":: nested\n");
    write_file(temp.path(), "target/ignored.recite", ":: ignored\n");
    write_file(temp.path(), ".hidden/ignored.recite", ":: ignored\n");

    let workspace = LspWorkspace::new(WorkspaceConfig::for_roots(vec![temp.path().to_owned()]));
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

pub(super) fn open_summary_overlays_saved_project_summary() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let source = temp.path().join("scene.recite");
    write_file(temp.path(), "scene.recite", ":: saved\n");

    let mut workspace = LspWorkspace::new(WorkspaceConfig::for_roots(vec![temp.path().to_owned()]));
    assert_eq!(block_names(&workspace), ["saved"]);

    workspace.open(file_uri(&source), 1, ":: live\n".to_owned());

    assert_eq!(block_names(&workspace), ["live"]);
    assert_eq!(workspace.snapshot().summaries()[0].version, Some(1));
}

pub(super) fn did_save_rekeys_new_open_file_without_duplicate_summary() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let source = temp.path().join("draft.recite");
    let uri = file_uri(&source);
    let mut workspace = LspWorkspace::new(WorkspaceConfig::for_roots(vec![temp.path().to_owned()]));

    workspace.open(uri.clone(), 1, ":: live\n".to_owned());
    assert_eq!(workspace.snapshot().summaries().len(), 1);
    assert!(
        workspace.snapshot().summaries()[0]
            .project_relative_path()
            .is_none()
    );

    write_file(temp.path(), "draft.recite", ":: saved\n");
    workspace.save(uri);

    assert_eq!(block_names(&workspace), ["live"]);
    let summaries = workspace.snapshot().summaries();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].version, Some(1));
    assert_eq!(summaries[0].project_relative_path(), Some("draft.recite"));
    assert!(summaries[0].saved_path().is_some());
}

pub(super) fn did_save_refreshes_saved_summary_for_closed_files() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let source = temp.path().join("scene.recite");
    write_file(temp.path(), "scene.recite", ":: saved\n");
    let harness = harness_for_root(temp.path());

    write_file(temp.path(), "scene.recite", "oops\n:: saved\n");
    harness.did_save(file_uri(&source));
    let published = harness.recv_publish_diagnostics();

    assert!(
        published
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_ref()
                == Some(&NumberOrString::String("RECITE_PARSE001".to_owned())))
    );

    harness.finish();
}

pub(super) fn did_close_refreshes_saved_summary_before_falling_back() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let source = temp.path().join("scene.recite");
    write_file(temp.path(), "scene.recite", "oops\n:: saved\n");
    let harness = harness_for_root(temp.path());
    let uri = file_uri(&source);

    harness.did_open(
        uri.clone(),
        1,
        ":: live default\n> intro@b769cd02ad888d04dc53\n  Hello.\n",
    );
    assert!(harness.recv_publish_diagnostics().diagnostics.is_empty());

    harness.did_close(uri);
    let published = harness.recv_publish_diagnostics();

    assert_eq!(published.version, None);
    assert!(
        published
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_ref()
                == Some(&NumberOrString::String("RECITE_PARSE001".to_owned())))
    );

    harness.finish();
}

pub(super) fn schema_load_failure_keeps_source_only_snapshot() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "scene.recite", ":: source\n");
    write_file(temp.path(), "schema.json", r#"{"schema_version":"one"}"#);

    let workspace = LspWorkspace::new(
        WorkspaceConfig::for_roots(vec![temp.path().to_owned()])
            .with_schema_path(temp.path().join("schema.json")),
    );

    assert!(workspace.schema().summary().is_none());
    assert!(!workspace.schema().diagnostics().is_empty());
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
    let mut workspace = LspWorkspace::new(WorkspaceConfig::for_roots(Vec::new()));
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
