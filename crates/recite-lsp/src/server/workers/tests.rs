use super::{AnalysisJob, Epochs, Update, analyze};
use crate::workspace::{LspWorkspace, WorkspaceConfig};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, InitializeParams,
    TextDocumentContentChangeEvent, TextDocumentIdentifier, Uri, VersionedTextDocumentIdentifier,
};
use recite_compiler::authoring::CancellationToken;
use recite_ui::{UiCatalog, UiLocale};
use std::sync::Arc;

mod publication_model;

#[test]
fn update_batch_projects_final_versions_once_in_first_seen_uri_order() {
    let (workspace, catalog) = workspace_with_open_documents();
    let changed: Uri = "untitled:z".parse().unwrap();
    let sibling: Uri = "untitled:a".parse().unwrap();
    let job = AnalysisJob {
        through: 2,
        epochs: Epochs::default(),
        updates: vec![
            Arc::new(change(changed.clone(), 2, "oops\n:: changed\n")),
            Arc::new(change(changed.clone(), 3, ":: changed\n-> END\n")),
        ],
        refresh_uris: Vec::new(),
        control: CancellationToken::new(),
    };

    let snapshot = analyze(
        &InitializeParams::default(),
        &catalog,
        Some(&workspace),
        &job,
    )
    .unwrap()
    .unwrap();

    assert_eq!(snapshot.workspace.open_version(&changed), Some(3));
    assert_eq!(snapshot.diagnostics.len(), 2);
    assert_eq!(snapshot.diagnostics[0].uri, changed);
    assert_eq!(snapshot.diagnostics[0].version, Some(3));
    assert!(snapshot.diagnostics[0].diagnostics.is_empty());
    assert_eq!(snapshot.diagnostics[1].uri, sibling);
    assert_eq!(snapshot.diagnostics[1].version, Some(1));
    assert!(snapshot.diagnostics[1].diagnostics.is_empty());
}

#[test]
fn close_invalidations_keep_requested_uri_before_remaining_open_documents() {
    let (mut workspace, _) = workspace_with_open_documents();
    let closed: Uri = "untitled:z".parse().unwrap();
    let sibling: Uri = "untitled:a".parse().unwrap();
    let update = Update::Close(DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier {
            uri: closed.clone(),
        },
    });

    let affected: Vec<Uri> = update.apply(&mut workspace);

    assert!(update.is_applied(&workspace));
    assert_eq!(affected, vec![closed.clone(), sibling.clone()]);
    assert_eq!(workspace.open_version(&closed), None);
    assert_eq!(workspace.open_version(&sibling), Some(1));
}

fn workspace_with_open_documents() -> (LspWorkspace, Arc<UiCatalog>) {
    let catalog = Arc::new(UiCatalog::load(&UiLocale::default()).unwrap());
    let mut workspace = LspWorkspace::with_control(
        WorkspaceConfig::from_initialize_params(&InitializeParams::default()),
        Arc::clone(&catalog),
        CancellationToken::new(),
    )
    .unwrap();
    for name in ["a", "z"] {
        workspace.open_refreshes(
            format!("untitled:{name}").parse().unwrap(),
            1,
            format!(":: {name}\n-> END\n"),
        );
    }
    (workspace, catalog)
}

fn change(uri: Uri, version: i32, text: &str) -> Update {
    Update::Change(DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier { uri, version },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: text.to_owned(),
        }],
    })
}
