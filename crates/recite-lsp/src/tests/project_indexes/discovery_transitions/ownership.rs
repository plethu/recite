use serde_json::json;
use tempfile::TempDir;

use crate::workspace::{LspWorkspace, WorkspaceConfig};

use super::super::super::support::{block_names, file_uri, write_file};

pub(crate) fn all() {
    #[cfg(unix)]
    {
        symlink_alias_replacement_reconciles_canonical_identity();
        source_ownership_is_order_independent();
        canonical_symlink_excludes_apply_to_refreshes();
        symlink_identity_replacements_remove_stale_documents();
    }
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

#[cfg(unix)]
pub(crate) fn symlink_identity_replacements_remove_stale_documents() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let alias = temp.path().join("alias.recite");
    let target = temp.path().join("target.recite");
    let alias_uri = file_uri(&alias);
    let workspace_config = WorkspaceConfig::for_roots(vec![temp.path().to_owned()]);
    let mut workspace = LspWorkspace::new(workspace_config);

    write_file(temp.path(), "target.recite", ":: target\n");
    symlink(&target, &alias).expect("target alias");
    workspace.refresh_watched_uri(&alias_uri);
    assert_eq!(block_names(&workspace), ["target"]);

    std::fs::remove_file(&alias).expect("remove alias");
    write_file(temp.path(), "alias.recite", ":: regular\n");
    workspace.refresh_watched_uri(&alias_uri);
    assert_eq!(block_names(&workspace), ["regular"]);

    std::fs::remove_file(&alias).expect("remove regular alias");
    std::fs::remove_file(&target).expect("remove file target");
    std::fs::create_dir(&target).expect("directory target");
    symlink(&target, &alias).expect("directory alias");
    workspace.refresh_watched_uri(&alias_uri);
    assert!(workspace.snapshot().summaries().is_empty());

    std::fs::remove_file(&alias).expect("remove directory alias");
    std::fs::remove_dir(&target).expect("remove directory target");
    write_file(temp.path(), "target.txt", ":: text\n");
    symlink(temp.path().join("target.txt"), &alias).expect("non-rec ite alias");
    workspace.refresh_watched_uri(&alias_uri);
    assert!(workspace.snapshot().summaries().is_empty());
}
