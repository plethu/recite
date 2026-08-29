use serde_json::json;
use tempfile::TempDir;

use crate::workspace::{DiagnosticRefresh, LspWorkspace, WorkspaceConfig};

use super::super::support::{block_names, file_uri, write_file};

pub(crate) fn all() {
    malformed_manifest_stays_fail_closed_across_file_lifecycle();
    manifestless_refresh_preserves_discovery_candidate();
    nested_discovery_start_survives_manifest_transitions();
    manifestless_multi_root_documents_keep_project_relative_keys();
    multi_root_documents_keep_project_relative_keys();
    #[cfg(unix)]
    symlink_alias_replacement_reconciles_canonical_identity();
    manifest_refresh_clears_removed_saved_diagnostics_only();
    manifestless_watcher_keeps_builtin_exclusions();
    #[cfg(unix)]
    {
        source_ownership_is_order_independent();
        canonical_symlink_excludes_apply_to_refreshes();
    }
}

pub(crate) fn manifestless_watcher_keeps_builtin_exclusions() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "source.recite", ":: source\n");
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = LspWorkspace::new(WorkspaceConfig::from_initialize_params(&params));

    write_file(temp.path(), "target/new.recite", ":: target\n");
    write_file(temp.path(), ".hidden/new.recite", ":: hidden\n");
    workspace.refresh_watched_uri(&file_uri(&temp.path().join("target/new.recite")));
    workspace.refresh_watched_uri(&file_uri(&temp.path().join(".hidden/new.recite")));
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        write_file(temp.path(), "generated/linked.recite", ":: linked\n");
        symlink(
            temp.path().join("generated/linked.recite"),
            temp.path().join("link.recite"),
        )
        .expect("generated alias");
        workspace.refresh_watched_uri(&file_uri(&temp.path().join("link.recite")));

        write_file(temp.path(), "visible.recite", ":: visible\n");
        symlink(
            temp.path().join("visible.recite"),
            temp.path().join(".hidden/link.recite"),
        )
        .expect("hidden alias");
        workspace.refresh_watched_uri(&file_uri(&temp.path().join(".hidden/link.recite")));
    }
    assert_eq!(block_names(&workspace), ["source"]);
}

#[cfg(unix)]
pub(crate) fn source_ownership_is_order_independent() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "one.recite", ":: one\n");
    let alias = temp.path().join("alias.recite");
    let alias_uri = file_uri(&alias);
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = LspWorkspace::new(WorkspaceConfig::from_initialize_params(&params));

    // Alias-first creation and deletion must leave the independently owned
    // canonical source intact.
    symlink(temp.path().join("one.recite"), &alias).expect("alias");
    workspace.refresh_watched_uri(&alias_uri);
    std::fs::remove_file(&alias).expect("remove alias");
    workspace.refresh_watched_uri(&alias_uri);
    assert_eq!(block_names(&workspace), ["one"]);

    // Target-first refresh must also preserve the alias ownership transition.
    symlink(temp.path().join("one.recite"), &alias).expect("alias again");
    workspace.refresh_watched_uri(&alias_uri);
    write_file(temp.path(), "one.recite", ":: changed\n");
    workspace.refresh_watched_uri(&file_uri(&temp.path().join("one.recite")));
    std::fs::remove_file(&alias).expect("remove alias again");
    workspace.refresh_watched_uri(&alias_uri);
    assert_eq!(block_names(&workspace), ["changed"]);

    // A direct target deletion invalidates the canonical document even when a
    // stale alias ownership has not emitted its own watcher event.
    symlink(temp.path().join("one.recite"), &alias).expect("alias third time");
    std::fs::remove_file(temp.path().join("one.recite")).expect("remove target");
    workspace.refresh_watched_uri(&file_uri(&temp.path().join("one.recite")));
    assert!(workspace.snapshot().summaries().is_empty());

    // The alias can repopulate the document only after its target is valid.
    write_file(temp.path(), "one.recite", ":: restored\n");
    workspace.refresh_watched_uri(&alias_uri);
    assert_eq!(block_names(&workspace), ["restored"]);
}

#[cfg(unix)]
pub(crate) fn canonical_symlink_excludes_apply_to_refreshes() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n[discovery]\nexcludes = [\"generated/**\"]\n",
    );
    write_file(temp.path(), "kept.recite", ":: kept\n");
    write_file(temp.path(), "generated/ignored.recite", ":: ignored\n");
    symlink(
        temp.path().join("generated/ignored.recite"),
        temp.path().join("alias.recite"),
    )
    .expect("generated alias");
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = LspWorkspace::new(WorkspaceConfig::from_initialize_params(&params));
    assert_eq!(block_names(&workspace), ["kept"]);

    workspace.refresh_watched_uri(&file_uri(&temp.path().join("alias.recite")));
    assert_eq!(block_names(&workspace), ["kept"]);
}

pub(crate) fn malformed_manifest_stays_fail_closed_across_file_lifecycle() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "recite.project.toml", "format_version = [\n");
    let source = temp.path().join("source.recite");
    write_file(temp.path(), "source.recite", ":: saved\n");
    let manifest_uri = file_uri(&temp.path().join("recite.project.toml"));
    let source_uri = file_uri(&source);
    let mut workspace = LspWorkspace::new(WorkspaceConfig::from_initialize_params(
        &serde_json::from_value(json!({
            "rootUri": file_uri(temp.path()).as_str(),
            "capabilities": {},
        }))
        .expect("initialize params"),
    ));

    workspace.open(source_uri.clone(), 1, ":: overlay\n".to_owned());
    write_file(temp.path(), "source.recite", ":: changed\n");
    workspace.save(source_uri.clone());
    workspace.refresh_watched_uri(&source_uri);
    workspace.save(manifest_uri);
    assert_eq!(block_names(&workspace), ["overlay"]);
    workspace.close(source_uri);
    assert!(workspace.snapshot().summaries().is_empty());
}

pub(crate) fn manifestless_refresh_preserves_discovery_candidate() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "source.recite", ":: source\n");
    let manifest = temp.path().join("recite.project.toml");
    let manifest_uri = file_uri(&manifest);
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = LspWorkspace::new(WorkspaceConfig::from_initialize_params(&params));
    assert_eq!(block_names(&workspace), ["source"]);

    write_file(temp.path(), "recite.project.toml", "format_version = 1\n");
    workspace.refresh_watched_uri(&manifest_uri);
    assert_eq!(block_names(&workspace), ["source"]);

    std::fs::remove_file(&manifest).expect("remove manifest");
    workspace.refresh_watched_uri(&manifest_uri);
    assert_eq!(block_names(&workspace), ["source"]);

    write_file(temp.path(), "recite.project.toml", "format_version = 1\n");
    workspace.refresh_watched_uri(&manifest_uri);
    assert_eq!(block_names(&workspace), ["source"]);
}

pub(crate) fn manifestless_multi_root_documents_keep_project_relative_keys() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    write_file(temp.path(), "first/a.recite", ":: first\n");
    write_file(temp.path(), "second/a.recite", ":: second\n");
    let config = WorkspaceConfig::for_roots(vec![first.clone(), second.clone()]);
    let mut workspace = LspWorkspace::new(config);

    workspace.open(
        file_uri(&first.join("a.recite")),
        1,
        ":: open first\n".to_owned(),
    );
    workspace.open(
        file_uri(&second.join("a.recite")),
        1,
        ":: open second\n".to_owned(),
    );
    write_file(temp.path(), "first/a.recite", ":: saved first\n");
    write_file(temp.path(), "second/a.recite", ":: saved second\n");
    workspace.save(file_uri(&first.join("a.recite")));
    workspace.save(file_uri(&second.join("a.recite")));

    let keys = workspace
        .snapshot()
        .summaries()
        .iter()
        .filter_map(|summary| summary.project_relative_path())
        .collect::<Vec<_>>();
    assert_eq!(keys, ["first/a.recite", "second/a.recite"]);
}

pub(crate) fn nested_discovery_start_survives_manifest_transitions() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let nested = temp.path().join("nested");
    write_file(temp.path(), "nested/a.recite", ":: nested\n");
    let parent_manifest = temp.path().join("recite.project.toml");
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"nested\"]\n",
    );
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(&nested).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = LspWorkspace::new(WorkspaceConfig::from_initialize_params(&params));
    assert_eq!(block_names(&workspace), ["nested"]);

    workspace.refresh_watched_uri(&file_uri(&parent_manifest));
    std::fs::remove_file(&parent_manifest).expect("remove parent manifest");
    workspace.refresh_watched_uri(&file_uri(&parent_manifest));

    let nested_manifest = nested.join("recite.project.toml");
    write_file(
        temp.path(),
        "nested/recite.project.toml",
        "format_version = 1\n",
    );
    workspace.refresh_watched_uri(&file_uri(&nested_manifest));
    let keys = workspace
        .snapshot()
        .summaries()
        .iter()
        .filter_map(|summary| summary.project_relative_path())
        .collect::<Vec<_>>();
    assert_eq!(keys, ["a.recite"]);

    std::fs::remove_file(&nested_manifest).expect("remove nested manifest");
    workspace.refresh_watched_uri(&file_uri(&nested_manifest));
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"nested\"]\n",
    );
    workspace.refresh_watched_uri(&file_uri(&parent_manifest));
    let keys = workspace
        .snapshot()
        .summaries()
        .iter()
        .filter_map(|summary| summary.project_relative_path())
        .collect::<Vec<_>>();
    assert_eq!(keys, ["nested/a.recite"]);
}

pub(crate) fn multi_root_documents_keep_project_relative_keys() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"src\", \"other\"]\n",
    );
    let src = temp.path().join("src/a.recite");
    let other = temp.path().join("other/a.recite");
    write_file(temp.path(), "src/a.recite", ":: src\n");
    write_file(temp.path(), "other/a.recite", ":: other\n");
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = LspWorkspace::new(WorkspaceConfig::from_initialize_params(&params));
    workspace.open(file_uri(&src), 1, ":: live src\n".to_owned());
    workspace.open(file_uri(&other), 1, ":: live other\n".to_owned());
    let keys = workspace
        .snapshot()
        .summaries()
        .iter()
        .filter_map(|summary| summary.project_relative_path())
        .collect::<Vec<_>>();
    assert_eq!(keys, ["other/a.recite", "src/a.recite"]);
}

#[cfg(unix)]
pub(crate) fn symlink_alias_replacement_reconciles_canonical_identity() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "recite.project.toml", "format_version = 1\n");
    write_file(temp.path(), "one.recite", ":: one\n");
    write_file(temp.path(), "two.recite", ":: two\n");
    let alias = temp.path().join("alias.recite");
    symlink(temp.path().join("one.recite"), &alias).expect("initial alias");
    let uri = file_uri(&alias);
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = LspWorkspace::new(WorkspaceConfig::from_initialize_params(&params));
    workspace.save(uri.clone());

    std::fs::remove_file(&alias).expect("remove alias");
    symlink(temp.path().join("two.recite"), &alias).expect("replacement alias");
    workspace.save(uri);
    let keys = workspace
        .snapshot()
        .summaries()
        .iter()
        .filter_map(|summary| summary.project_relative_path())
        .collect::<Vec<_>>();
    assert_eq!(keys, ["one.recite", "two.recite"]);
}

pub(crate) fn manifest_refresh_clears_removed_saved_diagnostics_only() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "recite.project.toml", "format_version = 1\n");
    let closed = temp.path().join("closed.recite");
    let open = temp.path().join("open.recite");
    write_file(temp.path(), "closed.recite", "oops\n");
    write_file(temp.path(), "open.recite", "oops\n");
    let closed_uri = file_uri(&closed);
    let open_uri = file_uri(&open);
    let manifest_uri = file_uri(&temp.path().join("recite.project.toml"));
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = LspWorkspace::new(WorkspaceConfig::from_initialize_params(&params));
    workspace.open(open_uri.clone(), 1, ":: overlay\n".to_owned());
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"other\"]\n",
    );
    std::fs::create_dir(temp.path().join("other")).expect("other root");
    let refreshes = workspace.save(manifest_uri);
    assert!(refreshes.iter().any(|refresh| matches!(
        refresh,
        DiagnosticRefresh::Clear { uri, .. } if uri == &closed_uri
    )));
    assert!(!refreshes.iter().any(|refresh| matches!(
        refresh,
        DiagnosticRefresh::Clear { uri, .. } if uri == &open_uri
    )));
    assert_eq!(block_names(&workspace), ["overlay"]);
}
