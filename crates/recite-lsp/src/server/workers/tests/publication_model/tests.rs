use super::*;
use crate::paths::file_path_to_uri;
use crate::server::freshness::Epochs;
use crate::server::workers::{AnalysisJob, analyze};
use crate::workspace::{DiagnosticRefresh, LspWorkspace, WorkspaceConfig};
use lsp_types::{DidChangeWatchedFilesParams, FileChangeType, FileEvent, InitializeParams};
use recite_compiler::authoring::CancellationToken;
use recite_ui::{UiCatalog, UiLocale};
use std::collections::BTreeSet;
use std::sync::Arc;

proptest! {
    #[test]
    fn editor_batches_preserve_uri_versions_and_complete_publications(
        batches in proptest::collection::vec(
            proptest::collection::vec(actions(), 1..5), 1..6,
        ),
    ) {
        let catalog = Arc::new(UiCatalog::load(&UiLocale::default()).unwrap());
        let mut workspace = empty_workspace(Arc::clone(&catalog));
        let mut documents = Documents::default();
        let mut model = EditorModel::default();
        // Every generated history starts with two distinct live buffers,
        // including a real diagnostic that a subsequent close must clear.
        let mut initial_updates = Vec::new();
        for (document, broken) in [(0, true), (1, false)] {
            let action = Action { document, kind: ActionKind::Open, version: 1, broken };
            prop_assert!(model.accept(&action));
            let mut update = action.update();
            prop_assert!(documents.accept(&mut update));
            initial_updates.push(Arc::new(update));
        }
        let initial = analyze(&InitializeParams::default(), &catalog, Some(&workspace),
            &analysis_job(initial_updates)).unwrap().unwrap();
        prop_assert_eq!(publication_map(initial.diagnostics), model.publications(0..2));
        workspace = initial.workspace;
        for batch in batches {
            let initially_open = model.0.keys().copied().collect::<BTreeSet<_>>();
            let mut affected = BTreeSet::new();
            let mut accepted = Vec::new();
            for action in batch {
                let expected_acceptance = model.accept(&action);
                let mut update = action.update();
                prop_assert_eq!(documents.accept(&mut update), expected_acceptance, "{:?}", action);
                if expected_acceptance {
                    affected.extend(&initially_open);
                    affected.insert(action.document);
                    accepted.push(Arc::new(update));
                }
            }
            let job = analysis_job(accepted);
            let snapshot = analyze(&InitializeParams::default(), &catalog, Some(&workspace), &job)
                .unwrap().unwrap();
            prop_assert_eq!(publication_map(snapshot.diagnostics), model.publications(affected));
            assert_editor_snapshot(&snapshot.workspace, &model);
            workspace = snapshot.workspace;
        }
    }

    #[test]
    fn watched_event_batches_publish_every_uri_from_final_filesystem_state(
        batches in proptest::collection::vec(
            proptest::collection::vec((0..DOCUMENT_COUNT, file_actions()), 1..4), 1..5,
        ),
    ) {
        let directory = tempfile::tempdir().unwrap();
        // A permanent entry keeps project-wide default-block validation
        // independent of the generated files' deletion/recreation lifecycle.
        std::fs::write(directory.path().join("entry.recite"), ":: entry default\n-> END\n").unwrap();
        let paths = (0..DOCUMENT_COUNT).map(|document| {
            let path = directory.path().join(format!("buffer-{document}.recite"));
            std::fs::write(&path, source(document, false)).unwrap();
            path
        }).collect::<Vec<_>>();
        let catalog = Arc::new(UiCatalog::load(&UiLocale::default()).unwrap());
        let mut workspace = Arc::new(LspWorkspace::with_control(
            WorkspaceConfig::for_roots(vec![directory.path().to_owned()]),
            Arc::clone(&catalog), CancellationToken::new(),
        ).unwrap());
        let mut model = [Some(false); DOCUMENT_COUNT];
        for batch in batches {
            let mut affected = BTreeSet::new();
            let mut events = Vec::new();
            for (document, action) in batch {
                let typ = match action {
                    FileAction::Delete => {
                        if model[document].is_some() {
                            std::fs::remove_file(&paths[document]).unwrap();
                        }
                        model[document] = None;
                        FileChangeType::DELETED
                    }
                    FileAction::Write(broken) => {
                        let typ = if model[document].is_some() {
                            FileChangeType::CHANGED
                        } else {
                            FileChangeType::CREATED
                        };
                        std::fs::write(&paths[document], source(document, broken)).unwrap();
                        model[document] = Some(broken);
                        typ
                    }
                };
                affected.insert(document);
                events.push(FileEvent { uri: file_path_to_uri(&paths[document]).unwrap(), typ });
            }
            let job = analysis_job(vec![Arc::new(Update::Watch(DidChangeWatchedFilesParams {
                changes: events,
            }))]);
            let snapshot = analyze(&InitializeParams::default(), &catalog, Some(&workspace), &job)
                .unwrap().unwrap();
            let expected = affected.into_iter().map(|document| {
                let publication = expected_publication(
                    file_path_to_uri(&paths[document]).unwrap(), model[document] == Some(true), None,
                );
                (publication.uri.as_str().to_owned(), publication)
            }).collect::<BTreeMap<_, _>>();
            prop_assert_eq!(publication_map(snapshot.diagnostics), expected);
            workspace = snapshot.workspace;
        }
    }
}

fn publication_map(
    publications: Vec<PublishDiagnosticsParams>,
) -> BTreeMap<String, PublishDiagnosticsParams> {
    let count = publications.len();
    let by_uri = publications
        .into_iter()
        .map(|publication| (publication.uri.as_str().to_owned(), publication))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(count, by_uri.len(), "duplicate URI publication");
    by_uri
}

fn empty_workspace(catalog: Arc<UiCatalog>) -> Arc<LspWorkspace> {
    Arc::new(
        LspWorkspace::with_control(
            WorkspaceConfig::from_initialize_params(&InitializeParams::default()),
            catalog,
            CancellationToken::new(),
        )
        .unwrap(),
    )
}

fn analysis_job(updates: Vec<Arc<Update>>) -> AnalysisJob {
    AnalysisJob {
        through: 1,
        epochs: Epochs::default(),
        updates,
        refresh_uris: Vec::new(),
        control: CancellationToken::new(),
    }
}

fn assert_editor_snapshot(workspace: &LspWorkspace, model: &EditorModel) {
    for document in 0..DOCUMENT_COUNT {
        let uri = editor_uri(document);
        let state = model.0.get(&document);
        assert_eq!(
            workspace.open_version(&uri),
            state.map(|state| state.version)
        );
        let refresh = workspace.diagnostic_refresh_for_uri(&uri);
        assert!(workspace.is_current_generation(refresh.generation()));
        match (state, refresh) {
            (Some(state), DiagnosticRefresh::Publish(published)) => {
                assert_eq!(published.uri, uri);
                assert_eq!(published.text, state.text);
                assert_eq!(published.version, Some(state.version));
            }
            (
                None,
                DiagnosticRefresh::Clear {
                    uri: cleared,
                    version,
                    ..
                },
            ) => {
                assert_eq!(cleared, uri);
                assert_eq!(version, None);
            }
            other => panic!("publication ownership diverged: {other:?}"),
        }
    }
}
