use lsp_types::{
    DocumentChanges, GotoDefinitionResponse, OneOf, Position, PrepareRenameResponse, Range,
    TextEdit, Uri,
};
use tempfile::TempDir;

use super::support::{Harness, file_uri, full_change, harness_for_root, uri, write_file};

pub(super) fn definition_resolves_block_references() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/navigation.recite");
    harness.did_open(
        source_uri.clone(),
        1,
        concat!(
            ":: start default\n",
            "> intro@8a535b2e538dd4f39758\n",
            "  Hello.\n",
            "-> target\n",
            ":: target\n",
            "> target_line@b7cf36a63a75edb16a8f\n",
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

pub(super) fn typed_features_follow_open_overlay_generation() {
    let initial = ":: start default\r\n-> stale_target\r\n:: stale_target\r\n";
    let temp = TempDir::new().expect("tempdir");
    write_file(temp.path(), "overlay-navigation.recite", initial);
    let source_uri = file_uri(&temp.path().join("overlay-navigation.recite"));
    let mut harness = harness_for_root(temp.path());
    harness.did_open(source_uri.clone(), 1, initial);
    let _ = harness.recv_publish_diagnostics();

    let initial_completion = harness
        .completion(source_uri.clone(), position_after(initial, "-> st"))
        .expect("schema-free block completion");
    assert_eq!(
        completion_labels(initial_completion),
        ["stale_target", "start"]
    );

    let updated = concat!(
        ":: start default\r\n",
        "> line@a1b2c3d4e5f60718293a\r\n",
        "  😀 overlay text.\r\n",
        "-> fresh_target\r\n",
        ":: fresh_target\r\n",
    );
    harness.did_change(source_uri.clone(), 2, vec![full_change(updated)]);
    let _ = harness.recv_publish_diagnostics();

    let fresh_position = position_inside(updated, "fresh_target");
    let updated_completion = harness
        .completion(source_uri.clone(), position_after(updated, "-> fresh"))
        .expect("updated schema-free block completion");
    assert_eq!(
        completion_labels(updated_completion),
        ["fresh_target", "start"]
    );

    let hover = harness
        .hover(source_uri.clone(), fresh_position)
        .expect("typed block hover from the open overlay");
    assert_eq!(hover.range, Some(range(3, 3, 3, 15)));

    let definition = harness
        .definition(source_uri.clone(), fresh_position)
        .expect("typed definition from the open overlay");
    let GotoDefinitionResponse::Scalar(location) = definition else {
        panic!("expected scalar definition response");
    };
    assert_eq!(location.uri, source_uri);
    assert_eq!(location.range, range(4, 3, 4, 15));

    let references = harness
        .references(source_uri.clone(), fresh_position, true)
        .expect("typed references from the open overlay");
    assert_eq!(
        references
            .iter()
            .map(|location| location.range)
            .collect::<Vec<_>>(),
        [range(4, 3, 4, 15), range(3, 3, 3, 15)]
    );
    let references_without_declaration = harness
        .references(source_uri.clone(), fresh_position, false)
        .expect("typed references without declaration");
    assert_eq!(
        references_without_declaration
            .iter()
            .map(|location| location.range)
            .collect::<Vec<_>>(),
        [range(3, 3, 3, 15)]
    );

    let prepare = harness
        .prepare_rename(source_uri.clone(), fresh_position)
        .expect("typed prepare rename from the open overlay");
    assert_eq!(
        prepare,
        PrepareRenameResponse::RangeWithPlaceholder {
            range: range(3, 3, 3, 15),
            placeholder: "fresh_target".to_owned(),
        }
    );
    let edit = harness
        .rename(source_uri.clone(), fresh_position, "renamed")
        .expect("compatibility rename projection from typed references");
    let Some(DocumentChanges::Edits(changes)) = edit.document_changes else {
        panic!("expected document changes");
    };
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
                range: range(3, 3, 3, 15),
                new_text: "renamed".to_owned(),
            },
            TextEdit {
                range: range(4, 3, 4, 15),
                new_text: "renamed".to_owned(),
            },
        ]
    );

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
    let temp = TempDir::new().expect("tempdir");
    let source = concat!(
        ":: start default\n",
        "> target@8392209a350039cc0dfd\n",
        "  This stable line ID must stay target.\n",
        "? choice_target@5a9d82b6cb8104fc9f19\n",
        "  This choice ID must stay choice_target.\n",
        "  -> target\n",
        ":: target\n",
        "> second@1a9463b9bc53e7500590\n",
        "  Done.\n",
    );
    write_file(temp.path(), "rename.recite", source);
    let source_uri = file_uri(&temp.path().join("rename.recite"));
    let mut harness = harness_for_root(temp.path());
    harness.did_open(source_uri.clone(), 1, source);
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
            "> line_id@6deabd0ba0f3a4d7938a\n",
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

fn completion_labels(response: lsp_types::CompletionResponse) -> Vec<String> {
    match response {
        lsp_types::CompletionResponse::Array(items) => {
            items.into_iter().map(|item| item.label).collect()
        }
        lsp_types::CompletionResponse::List(list) => {
            list.items.into_iter().map(|item| item.label).collect()
        }
    }
}

fn position_after(source: &str, needle: &str) -> Position {
    position_for_byte_index(
        source,
        source
            .find(needle)
            .unwrap_or_else(|| panic!("needle not found: {needle}"))
            + needle.len(),
    )
}

fn position_inside(source: &str, needle: &str) -> Position {
    position_for_byte_index(
        source,
        source
            .find(needle)
            .unwrap_or_else(|| panic!("needle not found: {needle}"))
            + 1,
    )
}

fn position_for_byte_index(source: &str, byte_index: usize) -> Position {
    let mut line = 0_u32;
    let mut character = 0_u32;
    for value in source[..byte_index].chars() {
        if value == '\n' {
            line = line.saturating_add(1);
            character = 0;
        } else {
            character = character.saturating_add(value.len_utf16() as u32);
        }
    }
    Position::new(line, character)
}
