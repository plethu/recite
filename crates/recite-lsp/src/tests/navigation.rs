use lsp_types::{
    DocumentChanges, GotoDefinitionResponse, OneOf, Position, PrepareRenameResponse, Range,
    TextEdit, Uri,
};
use tempfile::TempDir;

use super::support::{Harness, file_uri, harness_for_root, uri, write_file};

pub(super) fn definition_resolves_block_references() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/navigation.recite");
    harness.did_open(
        source_uri.clone(),
        1,
        concat!(
            ":: start default\n",
            "> intro\n",
            "  Hello.\n",
            "-> target\n",
            ":: target\n",
            "> target_line\n",
            "  There.\n",
        ),
    );
    let _ = harness.recv_publish_diagnostics();

    let definition = harness
        .definition(source_uri.clone(), Position::new(3, 5))
        .expect("definition response");
    let GotoDefinitionResponse::Scalar(location) = definition else {
        panic!("expected scalar definition response");
    };
    assert_eq!(location.uri, source_uri);
    assert_eq!(location.range, range(4, 3, 4, 9));

    harness.finish();
}

pub(super) fn references_include_declaration_and_project_references() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "a.recite", ":: start\n-> shared\n:: shared\n");
    write_file(temp.path(), "b.recite", ":: shared\n-> shared\n");
    write_file(
        temp.path(),
        "nested/c.recite",
        ":: caller\n-> b.recite::shared\n:: shared\n-> shared\n",
    );
    let mut harness = harness_for_root(temp.path());
    let local_uri = file_uri(&temp.path().join("a.recite"));
    let external_uri = file_uri(&temp.path().join("nested/c.recite"));

    let local_references = harness
        .references(local_uri, Position::new(1, 5), true)
        .expect("local references response");

    assert_eq!(
        local_references
            .iter()
            .map(|location| (relative_uri(&location.uri, temp.path()), location.range))
            .collect::<Vec<_>>(),
        [
            ("a.recite".to_owned(), range(2, 3, 2, 9)),
            ("a.recite".to_owned(), range(1, 3, 1, 9)),
        ]
    );

    let external_references = harness
        .references(external_uri, Position::new(1, 14), true)
        .expect("external references response");

    assert_eq!(
        external_references
            .iter()
            .map(|location| (relative_uri(&location.uri, temp.path()), location.range))
            .collect::<Vec<_>>(),
        [
            ("b.recite".to_owned(), range(0, 3, 0, 9)),
            ("b.recite".to_owned(), range(1, 3, 1, 9)),
            ("nested/c.recite".to_owned(), range(1, 13, 1, 19)),
        ]
    );

    harness.finish();
}

pub(super) fn rename_updates_only_block_symbols() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/rename.recite");
    harness.did_open(
        source_uri.clone(),
        1,
        concat!(
            ":: start default\n",
            "> target\n",
            "  This stable line ID must stay target.\n",
            "? choice_target\n",
            "  This choice ID must stay choice_target.\n",
            "  -> target\n",
            ":: target\n",
            "> second\n",
            "  Done.\n",
        ),
    );
    let _ = harness.recv_publish_diagnostics();

    let prepare = harness
        .prepare_rename(source_uri.clone(), Position::new(6, 4))
        .expect("prepare rename response");
    assert_eq!(
        prepare,
        PrepareRenameResponse::RangeWithPlaceholder {
            range: range(6, 3, 6, 9),
            placeholder: "target".to_owned(),
        }
    );

    let edit = harness
        .rename(source_uri.clone(), Position::new(6, 4), "renamed")
        .expect("rename edit");
    let Some(DocumentChanges::Edits(changes)) = edit.document_changes else {
        panic!("expected document changes");
    };
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].text_document.uri, source_uri);
    let edits = changes[0]
        .edits
        .iter()
        .map(|edit| match edit {
            OneOf::Left(edit) => edit.clone(),
            OneOf::Right(_) => panic!("expected plain text edit"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        edits,
        [
            TextEdit {
                range: range(5, 5, 5, 11),
                new_text: "renamed".to_owned(),
            },
            TextEdit {
                range: range(6, 3, 6, 9),
                new_text: "renamed".to_owned(),
            },
        ]
    );

    harness.finish();
}

pub(super) fn rename_rejects_non_block_symbols_and_invalid_names() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/reject-rename.recite");
    harness.did_open(
        source_uri.clone(),
        1,
        concat!(
            ":: start default\n",
            "> line_id\n",
            "  Hello.\n",
            "-> target\n",
            ":: target\n",
        ),
    );
    let _ = harness.recv_publish_diagnostics();

    assert!(
        harness
            .prepare_rename(source_uri.clone(), Position::new(1, 4))
            .is_none()
    );
    assert!(
        harness
            .rename(source_uri.clone(), Position::new(4, 4), "not valid")
            .is_none()
    );

    harness.finish();
}

fn range(start_line: u32, start_character: u32, end_line: u32, end_character: u32) -> Range {
    Range {
        start: Position::new(start_line, start_character),
        end: Position::new(end_line, end_character),
    }
}

fn relative_uri(uri: &Uri, root: &std::path::Path) -> String {
    let Some(path) = crate::paths::uri_to_file_path(uri) else {
        return uri.to_string();
    };
    path.strip_prefix(root)
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| uri.to_string())
}
