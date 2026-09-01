use lsp_types::NumberOrString;
use serde_json::json;
use tempfile::TempDir;

use crate::paths::stable_path_identity;
use crate::workspace::{DiagnosticRefresh, WorkspaceChangeResult, WorkspaceConfig};

use super::super::support::{
    block_names, file_uri, full_change, harness_for_root, test_workspace, write_file,
};

pub(crate) fn manifest_refresh_is_atomic_and_preserves_open_overlay() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let manifest = temp.path().join("recite.project.toml");
    write_file(temp.path(), "recite.project.toml", "format_version = 1\n");
    let source = temp.path().join("source.recite");
    write_file(temp.path(), "source.recite", ":: saved\n");
    let uri = file_uri(&source);
    let manifest_uri = file_uri(&manifest);
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(
        &serde_json::from_value(json!({
            "rootUri": file_uri(temp.path()).as_str(),
            "capabilities": {},
        }))
        .expect("initialize params"),
    ));

    workspace.open_refreshes(uri.clone(), 1, ":: overlay\n".to_owned());
    write_file(temp.path(), "recite.project.toml", "format_version = [\n");
    let refreshes = workspace.save(manifest_uri.clone());
    assert_eq!(refreshes.len(), 1);
    assert_eq!(block_names(&workspace), ["overlay"]);
    assert_eq!(workspace.snapshot().summaries().len(), 1);
    assert_eq!(workspace.snapshot().summaries()[0].version, Some(1));

    write_file(temp.path(), "recite.project.toml", "format_version = 1\n");
    workspace.save(manifest_uri);
    assert_eq!(block_names(&workspace), ["overlay"]);
    assert!(workspace.project_diagnostics_all().is_empty());
    workspace.close(uri);
    assert_eq!(block_names(&workspace), ["saved"]);
}

pub(crate) fn manifest_refresh_reuses_unchanged_sibling_kernel() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    write_file(&first, "src/main.recite", ":: first\n");
    write_file(&second, "src/main.recite", ":: second\n");
    let params = serde_json::from_value(json!({
        "workspaceFolders": [
            {"uri": file_uri(&first).as_str(), "name": "first"},
            {"uri": file_uri(&second).as_str(), "name": "second"}
        ],
        "capabilities": {}
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));
    let second_uri = file_uri(&second.join("src/main.recite"));
    workspace.open_refreshes(second_uri, 1, ":: second live\n".to_owned());
    let first_id = stable_path_identity(&first);
    let second_id = stable_path_identity(&second);
    let first_before = workspace
        .partition_kernel_generation(&first_id)
        .expect("first workspace partition");
    let second_before = workspace
        .partition_kernel_generation(&second_id)
        .expect("second workspace partition");

    write_file(
        &first,
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"src\"]\n",
    );
    workspace.refresh_watched_uri(&file_uri(&first.join("recite.project.toml")));

    assert_eq!(
        workspace.partition_kernel_generation(&second_id),
        Some(second_before),
        "manifest refresh must retain the untouched sibling kernel"
    );
    assert_ne!(
        workspace.partition_kernel_generation(&first_id),
        Some(first_before),
        "manifest refresh must rebuild the affected kernel"
    );
    let first_after = workspace
        .partition_kernel_generation(&first_id)
        .expect("rebuilt first workspace partition");

    workspace.exhaust_generation_for_test();
    workspace.refresh_watched_uri(&file_uri(&first.join("src/main.recite")));
    assert_eq!(
        workspace.partition_kernel_generation(&first_id),
        Some(first_after),
        "failed refresh must retain the affected partition identity"
    );
    assert_eq!(
        workspace.partition_kernel_generation(&second_id),
        Some(second_before),
        "failed refresh must retain the untouched sibling kernel"
    );
}

#[cfg(unix)]
pub(crate) fn saved_uri_replacement_removes_old_canonical_entry() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "recite.project.toml", "format_version = 1\n");
    write_file(temp.path(), "inside.recite", ":: inside\n");
    let outside = TempDir::new().unwrap_or_else(|error| panic!("outside: {error}"));
    write_file(outside.path(), "outside.recite", ":: outside\n");
    let link = temp.path().join("inside.recite");
    let uri = file_uri(&link);
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(
        &serde_json::from_value(json!({
            "rootUri": file_uri(temp.path()).as_str(),
            "capabilities": {},
        }))
        .expect("initialize params"),
    ));
    workspace.save(uri.clone());
    assert_eq!(workspace.snapshot().summaries().len(), 1);

    std::fs::remove_file(&link).expect("remove inside source");
    symlink(outside.path().join("outside.recite"), &link).expect("outside link");
    workspace.save(uri);
    assert!(workspace.snapshot().summaries().is_empty());
}

pub(crate) fn watched_files_refresh_saved_index_for_create_and_delete() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "recite.project.toml", "format_version = 1\n");
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));
    let source = temp.path().join("created.recite");
    write_file(temp.path(), "created.recite", ":: created\n");
    assert_eq!(workspace.refresh_watched_uri(&file_uri(&source)).len(), 1);
    assert_eq!(block_names(&workspace), ["created"]);

    std::fs::remove_file(&source).expect("remove source");
    assert_eq!(workspace.refresh_watched_uri(&file_uri(&source)).len(), 1);
    assert!(workspace.snapshot().summaries().is_empty());
}

pub(crate) fn open_summary_overlays_saved_project_summary() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let source = temp.path().join("scene.recite");
    write_file(temp.path(), "scene.recite", ":: saved\n");

    let mut workspace = test_workspace(WorkspaceConfig::for_roots(vec![temp.path().to_owned()]));
    assert_eq!(block_names(&workspace), ["saved"]);

    workspace.open_refreshes(file_uri(&source), 1, ":: live\n".to_owned());

    assert_eq!(block_names(&workspace), ["live"]);
    assert_eq!(workspace.snapshot().summaries()[0].version, Some(1));
}

pub(crate) fn did_save_rekeys_new_open_file_without_duplicate_summary() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let source = temp.path().join("draft.recite");
    let uri = file_uri(&source);
    let mut workspace = test_workspace(WorkspaceConfig::for_roots(vec![temp.path().to_owned()]));

    workspace.open_refreshes(uri.clone(), 1, ":: live\n".to_owned());
    assert_eq!(workspace.snapshot().summaries().len(), 1);
    assert_eq!(
        workspace.snapshot().summaries()[0].project_relative_path(),
        Some("draft.recite")
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

#[cfg(unix)]
pub(crate) fn open_nonexistent_aliases_share_one_fallback_key() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let real = temp.path().join("real");
    let alias = temp.path().join("alias");
    std::fs::create_dir(&real).expect("real directory");
    symlink(&real, &alias).expect("directory alias");
    let real_uri = file_uri(&real.join("draft.recite"));
    let alias_uri = file_uri(&alias.join("draft.recite"));
    let mut workspace = test_workspace(WorkspaceConfig::for_roots(vec![temp.path().to_owned()]));

    workspace.open_refreshes(real_uri, 1, ":: draft\n".to_owned());
    workspace.open_refreshes(alias_uri, 1, ":: draft\n".to_owned());

    assert_eq!(workspace.snapshot().summaries().len(), 1);
    assert_eq!(
        workspace.snapshot().summaries()[0].project_relative_path(),
        Some("real/draft.recite")
    );
}

pub(crate) fn did_save_refreshes_saved_summary_for_closed_files() {
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

pub(crate) fn did_close_refreshes_saved_summary_before_falling_back() {
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

#[cfg(unix)]
pub(crate) fn open_alias_owner_switch_reseeds_kernel_version_state() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let real_directory = temp.path().join("z");
    let alias_directory = temp.path().join("a");
    std::fs::create_dir(&real_directory).expect("real directory");
    symlink(&real_directory, &alias_directory).expect("directory alias");
    write_file(temp.path(), "z/draft.recite", ":: saved\n");
    let real_uri = file_uri(&real_directory.join("draft.recite"));
    let alias_uri = file_uri(&alias_directory.join("draft.recite"));
    let mut workspace = test_workspace(WorkspaceConfig::for_roots(vec![temp.path().to_owned()]));

    workspace.open_refreshes(real_uri.clone(), 10, ":: canonical\n".to_owned());
    assert_eq!(workspace.snapshot().summaries().len(), 1);
    assert_eq!(block_names(&workspace), ["canonical"]);
    assert_eq!(workspace.snapshot().summaries()[0].version, Some(10));

    // The alias sorts first and starts at an unrelated editor version. The
    // effective URI owner changes, so this must reseed kernel overlay state
    // instead of comparing version 1 with the canonical URI's version 10.
    workspace.open_refreshes(alias_uri.clone(), 1, ":: alias\n".to_owned());
    assert_eq!(workspace.snapshot().summaries().len(), 1);
    assert_eq!(block_names(&workspace), ["alias"]);
    assert_eq!(workspace.snapshot().summaries()[0].version, Some(1));

    assert!(matches!(
        workspace.change(alias_uri.clone(), 0, vec![full_change(":: stale\n")]),
        WorkspaceChangeResult::Stale
    ));
    let WorkspaceChangeResult::Accepted(_) = workspace.change(
        alias_uri.clone(),
        2,
        vec![full_change(":: alias-updated\n")],
    ) else {
        panic!("newer alias version should be accepted");
    };
    assert_eq!(workspace.snapshot().summaries().len(), 1);
    assert_eq!(block_names(&workspace), ["alias-updated"]);
    assert_eq!(workspace.snapshot().summaries()[0].version, Some(2));

    let refreshes = workspace.close(alias_uri);
    assert_eq!(workspace.snapshot().summaries().len(), 1);
    assert_eq!(block_names(&workspace), ["canonical"]);
    assert_eq!(workspace.snapshot().summaries()[0].version, Some(10));
    assert_eq!(refreshes.len(), 1);
    let DiagnosticRefresh::Publish(diagnostics) = &refreshes[0] else {
        panic!("closing alias should publish the remaining canonical overlay");
    };
    assert_eq!(diagnostics.uri, real_uri);
    assert_eq!(diagnostics.version, Some(10));
    assert_eq!(diagnostics.text, ":: canonical\n");
}

pub(crate) fn watched_refresh_publishes_effective_open_payload() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "scene.recite", ":: saved\n");
    let source = temp.path().join("scene.recite");
    let uri = file_uri(&source);
    let mut workspace = test_workspace(WorkspaceConfig::for_roots(vec![temp.path().to_owned()]));

    workspace.open_refreshes(uri.clone(), 7, "oops\n".to_owned());
    write_file(temp.path(), "scene.recite", ":: watched saved\n");

    let refreshes = workspace.refresh_watched_uri(&uri);
    assert_eq!(refreshes.len(), 1);
    let DiagnosticRefresh::Publish(diagnostics) = &refreshes[0] else {
        panic!("watched source refresh should publish diagnostics");
    };
    assert_eq!(diagnostics.uri, uri);
    assert_eq!(diagnostics.version, Some(7));
    assert_eq!(diagnostics.text, "oops\n");
    assert!(!diagnostics.diagnostics.is_empty());
}
