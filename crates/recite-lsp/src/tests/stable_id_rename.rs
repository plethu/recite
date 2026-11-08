use lsp_types::{DocumentChanges, OneOf, Position, PrepareRenameResponse, Range, TextEdit};
use recite_compiler::authoring::CancellationToken;
use tempfile::TempDir;

use crate::workspace::{LspWorkspace, WorkspaceConfig};

use super::support::{file_uri, test_workspace, write_file};

const SCENE: &str = concat!(
    ":: start default\n",
    "> speaker=mara @desk_greeting\n",
    "  Hello there.\n",
    "  ? @ask_room\n",
    "    I need a room.\n",
    "    -> END\n",
);

const FRENCH: &str = concat!(
    "msgctxt \"desk_greeting\"\nmsgid \"Hello there.\"\nmsgstr \"Bonjour.\"\n\n",
    "msgctxt \"desk_greeting&formal\"\nmsgid \"Hello there.\"\nmsgstr \"Bonjour, madame.\"\n\n",
    "#~ msgctxt \"retired_id\"\n#~ msgid \"Gone.\"\n#~ msgstr \"Parti.\"\n",
);

fn project() -> (TempDir, LspWorkspace) {
    let temp = TempDir::new().expect("tempdir");
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n\n[discovery]\ncatalogs = [\"locales/*.po\"]\n",
    );
    write_file(temp.path(), "dialogue/scene.recite", SCENE);
    let params = serde_json::from_value(serde_json::json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .expect("initialize params");
    let workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));
    (temp, workspace)
}

fn range(line: u32, start: u32, end: u32) -> Range {
    Range {
        start: Position::new(line, start),
        end: Position::new(line, end),
    }
}

fn rename(
    workspace: &LspWorkspace,
    temp: &TempDir,
    position: Position,
    new_name: &str,
) -> Result<Option<lsp_types::WorkspaceEdit>, String> {
    workspace
        .rename_with_control(
            &file_uri(&temp.path().join("dialogue/scene.recite")),
            position,
            new_name,
            &CancellationToken::new(),
        )
        .map_err(|refusal| refusal.message)
}

pub(super) fn prepare_rename_offers_the_id_without_its_marker() {
    let (temp, workspace) = project();
    let scene = file_uri(&temp.path().join("dialogue/scene.recite"));
    let control = CancellationToken::new();
    assert_eq!(
        workspace.prepare_rename_with_control(&scene, Position::new(1, 17), &control),
        Some(PrepareRenameResponse::RangeWithPlaceholder {
            range: range(1, 16, 29),
            placeholder: "desk_greeting".to_owned(),
        })
    );
    assert_eq!(
        workspace.prepare_rename_with_control(&scene, Position::new(2, 4), &control),
        None
    );
}

pub(super) fn rename_edits_the_header_and_catalogues_found_at_request_time() {
    let (temp, workspace) = project();
    // The catalogue is created after workspace discovery.
    write_file(temp.path(), "locales/fr.po", FRENCH);
    let catalogue = temp.path().join("locales/fr.po");

    let edit = rename(&workspace, &temp, Position::new(1, 20), "@front_desk_hello")
        .expect("rename is accepted")
        .expect("rename has edits");
    let Some(DocumentChanges::Edits(changes)) = edit.document_changes else {
        panic!("versioned document edits");
    };
    let edits = changes
        .iter()
        .map(|change| {
            (
                change.text_document.uri.clone(),
                change
                    .edits
                    .iter()
                    .map(|edit| match edit {
                        OneOf::Left(edit) => edit.clone(),
                        OneOf::Right(edit) => edit.text_edit.clone(),
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    let catalogue_edits = edits
        .iter()
        .find(|(uri, _)| *uri == file_uri(&catalogue))
        .map(|(_, edits)| edits.clone())
        .expect("catalogue is edited");
    assert_eq!(
        catalogue_edits,
        [
            TextEdit {
                range: range(0, 0, 23),
                new_text: "msgctxt \"front_desk_hello\"".to_owned(),
            },
            TextEdit {
                range: range(4, 0, 30),
                new_text: "msgctxt \"front_desk_hello&formal\"".to_owned(),
            },
        ]
    );
    let source_edits = edits
        .iter()
        .find(|(uri, _)| *uri == file_uri(&temp.path().join("dialogue/scene.recite")))
        .map(|(_, edits)| edits.clone())
        .expect("source is edited");
    assert_eq!(
        source_edits,
        [TextEdit {
            range: range(1, 15, 29),
            new_text: "@front_desk_hello".to_owned(),
        }]
    );
}

pub(super) fn rename_refusals_are_localised_messages() {
    let (temp, workspace) = project();
    write_file(temp.path(), "locales/fr.po", FRENCH);
    let refusal = |new_name| {
        rename(&workspace, &temp, Position::new(1, 20), new_name).expect_err("refused rename")
    };
    assert!(refusal("desk&formal").contains("`&`, which is reserved"));
    assert!(refusal("Desk").contains("is malformed"));
    assert!(refusal("ask_room").contains("already used by a line or choice"));
    assert!(refusal("retired_id").contains("catalogue locales/fr.po"));

    write_file(temp.path(), "locales/de.po", "msgid \"unterminated\n");
    assert!(refusal("fresh_name").contains("Catalogue locales/de.po is not valid PO"));
    assert_eq!(
        rename(&workspace, &temp, Position::new(1, 20), "desk_greeting"),
        Ok(None)
    );
}
