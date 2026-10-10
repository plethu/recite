use super::InputText;
use crate::paths::file_path_to_uri;
use crate::summary::{OpenFileIdentity, OpenFileScope};
use crate::workspace::{LspWorkspace, SnapshotGeneration, WorkspaceConfig};
use recite_compiler::authoring::AuthoringError;
use recite_config::UiLocale;
use recite_ui::UiCatalog;
use std::sync::Arc;

#[test]
fn input_text_identity_is_only_a_fast_path_for_exact_equality() {
    let text = InputText(Arc::from("Café 🦀\r\n"));
    let independent = InputText(Arc::from(text.0.to_string()));
    assert!(!Arc::ptr_eq(&text.0, &independent.0));
    assert_eq!(text, text.clone());
    assert_eq!(text, independent);
    assert_ne!(text, InputText(Arc::from("Café 🦀\n")));
    assert_ne!(text, InputText(Arc::from("Other")));
}

#[test]
fn interrupted_open_retains_committed_documents_and_snapshot() {
    let (directory, mut workspace) = workspace_with_saved_document();
    let previous = workspace.clone();
    workspace.control.interrupt();
    let uri = file_path_to_uri(&directory.path().join("draft.recite")).unwrap();

    assert!(
        workspace
            .open_refreshes(uri.clone(), 1, ":: draft\n".to_owned())
            .is_empty()
    );

    assert_eq!(workspace.open_version(&uri), None);
    assert_committed_workspace_preserved(&previous, &workspace);
}

#[test]
fn exhausted_workspace_generation_retains_committed_documents_and_snapshot() {
    let (directory, mut workspace) = workspace_with_saved_document();
    workspace.generation = SnapshotGeneration(u64::MAX);
    let previous = workspace.clone();
    let uri = file_path_to_uri(&directory.path().join("draft.recite")).unwrap();

    assert!(
        workspace
            .open_refreshes(uri.clone(), 1, ":: draft\n".to_owned())
            .is_empty()
    );

    assert_eq!(workspace.open_version(&uri), None);
    assert_committed_workspace_preserved(&previous, &workspace);
}

#[test]
fn exhausted_partition_generation_retains_committed_documents_and_snapshot() {
    let (directory, mut workspace) = workspace_with_saved_document();
    workspace.next_partition_build_id = u64::MAX;
    let previous = workspace.clone();
    let uri = file_path_to_uri(&directory.path().join("draft.recite")).unwrap();

    assert!(
        workspace
            .open_refreshes(uri.clone(), 1, ":: draft\n".to_owned())
            .is_empty()
    );

    assert_eq!(workspace.open_version(&uri), None);
    assert_committed_workspace_preserved(&previous, &workspace);
}

#[test]
fn failure_after_first_partition_candidate_retains_entire_committed_workspace() {
    let directory = tempfile::tempdir().unwrap();
    let roots = [
        directory.path().join("first"),
        directory.path().join("second"),
    ];
    for (index, root) in roots.iter().enumerate() {
        std::fs::create_dir(root).unwrap();
        std::fs::write(root.join("saved.recite"), format!(":: saved{index}\n")).unwrap();
    }
    let catalog = UiCatalog::load(&UiLocale::default()).unwrap();
    let mut workspace =
        LspWorkspace::with_ui_catalog(WorkspaceConfig::for_roots(roots.into()), catalog).unwrap();
    let mut documents = workspace.documents.clone();
    for (index, saved) in workspace.saved.documents.values().enumerate() {
        documents.open(
            OpenFileIdentity {
                uri: saved.identity.uri.clone(),
                saved_path: Some(saved.identity.canonical_path.clone()),
                project_relative_path: Some(saved.identity.project_relative_path.clone()),
                scope: OpenFileScope::Project,
            },
            1,
            format!(":: replacement{index}\n"),
        );
    }
    workspace.next_partition_build_id = u64::MAX - 1;
    let previous = workspace.clone();

    let result = workspace.rebuild_for_documents_with_schemas(
        workspace.saved.clone(),
        documents,
        workspace.partition_schemas(),
    );

    assert!(
        matches!(result, Err(AuthoringError::GenerationExhausted { current }) if current.as_u64() == u64::MAX),
        "the first changed partition must finish before the second build-ID overflow"
    );
    assert_committed_workspace_preserved(&previous, &workspace);
}

fn workspace_with_saved_document() -> (tempfile::TempDir, LspWorkspace) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("saved.recite"), ":: saved\n").unwrap();
    let catalog = UiCatalog::load(&UiLocale::default()).unwrap();
    let workspace = LspWorkspace::with_ui_catalog(
        WorkspaceConfig::for_roots(vec![directory.path().to_owned()]),
        catalog,
    )
    .unwrap();
    assert_eq!(workspace.snapshot.summaries().len(), 1);
    (directory, workspace)
}

fn assert_committed_workspace_preserved(previous: &LspWorkspace, workspace: &LspWorkspace) {
    assert_eq!(workspace.generation, previous.generation);
    assert_eq!(
        workspace.snapshot.generation(),
        previous.snapshot.generation()
    );
    assert_eq!(
        workspace.next_partition_build_id,
        previous.next_partition_build_id
    );
    assert_eq!(workspace.retired_schema_uris, previous.retired_schema_uris);
    assert_eq!(
        workspace.retired_schema_targets,
        previous.retired_schema_targets
    );
    assert!(Arc::ptr_eq(&workspace.query_index, &previous.query_index));
    assert!(std::ptr::eq(
        workspace.snapshot.summaries(),
        previous.snapshot.summaries()
    ));
    assert_eq!(workspace.partitions.len(), previous.partitions.len());
    assert_eq!(
        workspace.documents.documents().count(),
        previous.documents.documents().count()
    );
    for (id, partition) in &previous.partitions {
        let committed = workspace.partitions.get(id).unwrap();
        assert_eq!(committed.build_id, partition.build_id);
        assert!(Arc::ptr_eq(&committed.kernel, &partition.kernel));
        assert_eq!(committed.schema, partition.schema);
    }
}
