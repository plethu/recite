use std::os::unix::fs::symlink;

use lsp_types::{DocumentChanges, Position, Uri};
use recite_compiler::authoring::CancellationToken;
use tempfile::TempDir;

use crate::workspace::{LspWorkspace, WorkspaceChangeResult, WorkspaceConfig};

use super::super::support::{file_uri, full_change, test_workspace, write_file};

pub(crate) fn cursor_queries_match_the_requested_alias_buffer() {
    let temp = TempDir::new().expect("tempdir");
    let real_directory = temp.path().join("z");
    let alias_directory = temp.path().join("a");
    std::fs::create_dir(&real_directory).expect("real directory");
    symlink(&real_directory, &alias_directory).expect("directory alias");
    write_file(
        temp.path(),
        "z/draft.recite",
        ":: saved default\n-> saved\n",
    );
    let real = file_uri(&real_directory.join("draft.recite"));
    let alias = file_uri(&alias_directory.join("draft.recite"));
    let mut workspace = test_workspace(WorkspaceConfig::for_roots(vec![temp.path().to_owned()]));
    let real_text = ":: real default\n-> real\n";
    let alias_text = ":: alias default\n-> target\n:: target\n";
    workspace.open_refreshes(real.clone(), 10, real_text.to_owned());
    workspace.open_refreshes(alias.clone(), 1, alias_text.to_owned());

    let cursor = Position::new(1, 5);
    assert_queries(&workspace, &alias, cursor, true);
    assert_queries(&workspace, &real, cursor, false);
    // This column exists only in the canonical owner's buffer.
    assert_queries(&workspace, &real, Position::new(1, 8), false);

    assert!(matches!(
        workspace.change(real.clone(), 11, vec![full_change(alias_text)]),
        WorkspaceChangeResult::Accepted(_)
    ));
    // Distinct URI/version/allocation with identical bytes remains usable.
    assert_queries(&workspace, &real, cursor, true);
    assert_rename_owner(&workspace, &real, &alias, 1);
    assert!(matches!(
        workspace.change(real.clone(), 12, vec![full_change(real_text)]),
        WorkspaceChangeResult::Accepted(_)
    ));
    workspace.close(alias.clone());
    assert_queries(&workspace, &real, cursor, true);
    assert_rename_owner(&workspace, &real, &real, 12);

    workspace.open_refreshes(alias.clone(), 2, alias_text.to_owned());
    workspace.close(real.clone());
    // A saved URI must not interpret its coordinates in an open alias either.
    assert_queries(&workspace, &real, cursor, false);
    assert_queries(&workspace, &alias, cursor, true);
    workspace.close(alias);
    assert!(workspace.definition(&real, cursor).is_some());
    assert!(
        workspace
            .prepare_rename_with_control(&real, cursor, &CancellationToken::new())
            .is_some()
    );
}

fn assert_rename_owner(workspace: &LspWorkspace, requested: &Uri, owner: &Uri, version: i32) {
    let edit = workspace
        .rename_with_control(
            requested,
            Position::new(1, 5),
            "renamed",
            &CancellationToken::new(),
        )
        .expect("rename");
    let Some(DocumentChanges::Edits(changes)) = edit.document_changes else {
        panic!("versioned document edits");
    };
    assert_eq!(changes.len(), 1);
    assert_eq!(&changes[0].text_document.uri, owner);
    assert_eq!(changes[0].text_document.version, Some(version));
}

fn assert_queries(workspace: &LspWorkspace, uri: &Uri, cursor: Position, expected: bool) {
    let control = CancellationToken::new();
    for (name, actual) in [
        (
            "completion",
            workspace
                .completion_with_control(uri, cursor, &control)
                .is_some(),
        ),
        (
            "hover",
            workspace
                .hover_with_control(uri, cursor, &control)
                .is_some(),
        ),
        (
            "definition",
            workspace
                .definition_with_control(uri, cursor, &control)
                .is_some(),
        ),
        (
            "references",
            workspace
                .references_with_control(uri, cursor, true, &control)
                .is_some(),
        ),
        (
            "prepare rename",
            workspace
                .prepare_rename_with_control(uri, cursor, &control)
                .is_some(),
        ),
        (
            "rename",
            workspace
                .rename_with_control(uri, cursor, "renamed", &control)
                .is_some(),
        ),
    ] {
        assert_eq!(actual, expected, "{name}: {uri:?} at {cursor:?}");
    }
}
