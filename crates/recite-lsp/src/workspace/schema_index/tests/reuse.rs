use super::super::SchemaIndex;
use std::{fs, sync::Arc};

const TOML: &str = "schema_version = 1\n[producer]\nid = \"reuse-test\"\n[conditions.ready]\nparams = []\nreturns = \"bool\"\n";

#[test]
fn identical_overlay_bytes_share_payloads_and_update_exact_edit_authority() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schema.toml");
    fs::write(&path, TOML).unwrap();
    let saved = SchemaIndex::load(Some(path));
    let first = saved.overlay_for_open(saved.protocol_uri().unwrap(), TOML, 4);
    let alternate: lsp_types::Uri = "file:///alternate/schema.toml".parse().unwrap();
    let next = first.overlay_for_open(alternate.clone(), TOML, 8);
    assert!(Arc::ptr_eq(
        first.schema.as_ref().unwrap(),
        next.schema.as_ref().unwrap()
    ));
    assert!(Arc::ptr_eq(
        first.summary.as_ref().unwrap(),
        next.summary.as_ref().unwrap()
    ));
    assert!(Arc::ptr_eq(
        first.source.as_ref().unwrap(),
        next.source.as_ref().unwrap()
    ));
    assert!(Arc::ptr_eq(&first.diagnostics, &next.diagnostics));
    let edit = next.code_action_document().unwrap();
    assert_eq!(edit.uri, alternate);
    assert_eq!(edit.version, 8);
    assert_eq!(first.active_version, Some(4));
    assert_eq!(next.configured_uri, saved.configured_uri);
    let changed = first.overlay_for_open(
        first.protocol_uri().unwrap(),
        &TOML.replace("ready", "changed"),
        4,
    );
    assert!(!Arc::ptr_eq(
        first.schema.as_ref().unwrap(),
        changed.schema.as_ref().unwrap()
    ));
    assert!(
        changed
            .summary()
            .unwrap()
            .conditions()
            .iter()
            .any(|c| c.name() == "changed")
    );
}

#[test]
fn malformed_overlay_reuse_keeps_diagnostics_with_new_live_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schema.json");
    fs::write(&path, "{").unwrap();
    let saved = SchemaIndex::load(Some(path));
    let first = saved.overlay_for_open(saved.protocol_uri().unwrap(), "{", 1);
    let alternate: lsp_types::Uri = "file:///alternate/schema.json".parse().unwrap();
    let next = first.overlay_for_open(alternate.clone(), "{", 2);
    assert!(Arc::ptr_eq(&first.diagnostics, &next.diagnostics));
    let crate::workspace::DiagnosticRefresh::Publish(published) = next
        .diagnostics_refresh(crate::workspace::SnapshotGeneration(3))
        .unwrap()
    else {
        panic!("live diagnostics")
    };
    assert_eq!(published.uri, alternate);
    assert_eq!(published.version, Some(2));
    assert_eq!(published.diagnostics, first.diagnostics.as_ref());
    fs::remove_file(saved.configured_path().unwrap()).unwrap();
    let missing = saved.base();
    assert!(missing.text.is_none());
    let repaired =
        missing.overlay_for_open(missing.protocol_uri().unwrap(), "{\"schema_version\":1}", 3);
    assert!(repaired.schema().is_some());
    assert!(repaired.diagnostics.is_empty());
}

#[test]
fn manifest_refresh_reuses_closed_parse_but_rereads_disk_changes_and_failures() {
    use crate::workspace::{LspWorkspace, WorkspaceConfig};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schema.toml");
    fs::write(&path, TOML).unwrap();
    fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("scene.recite"),
        ":: start default\n-> END\n",
    )
    .unwrap();
    let config =
        WorkspaceConfig::for_roots(vec![dir.path().to_owned()]).with_schema_path(path.clone());
    let mut workspace = LspWorkspace::with_ui_catalog(
        config,
        recite_ui::UiCatalog::load(&recite_ui::UiLocale::default()).unwrap(),
    )
    .unwrap();
    let first = workspace
        .partition_schemas()
        .into_values()
        .find(|schema| schema.configured_path() == Some(path.as_path()))
        .unwrap();
    workspace.refresh_project_manifest();
    let next = workspace
        .partition_schemas()
        .into_values()
        .find(|schema| schema.configured_path() == Some(path.as_path()))
        .unwrap();
    assert!(Arc::ptr_eq(
        first.source.as_ref().unwrap(),
        next.source.as_ref().unwrap()
    ));
    for bytes in [
        Some(TOML.replace("ready", "changed")),
        Some("schema_version = [".into()),
        None,
        Some(TOML.into()),
    ] {
        match bytes {
            Some(text) => fs::write(&path, text).unwrap(),
            None => fs::remove_file(&path).unwrap(),
        }
        workspace.refresh_project_manifest();
        let actual = workspace
            .partition_schemas()
            .into_values()
            .find(|schema| schema.configured_path() == Some(path.as_path()))
            .unwrap();
        assert_eq!(actual, SchemaIndex::load(Some(path.clone())));
    }
}
