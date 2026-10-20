use recite_compiler::authoring::SavedDocument;
use recite_core::DocumentKey;
use recite_writer_model::{FIXTURE, ProjectContext, View, Workbench};
use recite_writer_session::{DocumentSession, FileError};
use std::{fs, path::PathBuf};

fn context(source: &str) -> Result<ProjectContext, Box<dyn std::error::Error>> {
    Ok(ProjectContext {
        documents: vec![SavedDocument::new(
            DocumentKey::new("scene.recite")?,
            source,
        )],
        schema: None,
    })
}
fn open(path: PathBuf) -> Result<DocumentSession, Box<dyn std::error::Error>> {
    Ok(DocumentSession::open(
        path,
        DocumentKey::new("scene.recite")?,
        context(FIXTURE)?,
    )?)
}

#[test]
fn queued_unicode_draft_survives_close_and_save_preserves_stable_identity() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scene.recite");
    fs::write(&path, FIXTURE).unwrap();
    let mut session = open(path.clone()).unwrap();
    let selected = session.model().view().clone();
    session
        .edit(|model| {
            model.set_draft("Café 日本語 مرحبا".into());
            Ok(())
        })
        .unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), FIXTURE);
    drop(session);
    let mut restored = open(path.clone()).unwrap();
    assert_eq!(restored.model().draft(), "Café 日本語 مرحبا");
    assert_eq!(restored.model().view(), &selected);
    restored.save().unwrap();
    assert!(!restored.dirty());
    assert_eq!(restored.model().view(), &selected);
    assert!(
        fs::read_to_string(&path)
            .unwrap()
            .contains("Café 日本語 مرحبا")
    );
    drop(restored);
    assert!(
        !path
            .with_file_name("scene.recite.recite-editor-recovery.json")
            .exists()
    );
}

#[test]
fn conflict_refuses_overwrite_and_reload_keeps_an_independent_recovery_copy() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scene.recite");
    fs::write(&path, FIXTURE).unwrap();
    let mut session = open(path.clone()).unwrap();
    let selected = session.model().view().clone();
    session
        .edit(|model| {
            model.set_draft("My local draft.".into());
            Ok(())
        })
        .unwrap();
    let external = format!("{FIXTURE}\n# An external edit\n");
    fs::write(&path, &external).unwrap();
    assert!(matches!(session.save(), Err(FileError::Conflict)));
    assert_eq!(fs::read_to_string(&path).unwrap(), external);
    let copy = session
        .reload(context(&external).unwrap())
        .unwrap()
        .unwrap();
    let recovery: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(copy).unwrap()).unwrap();
    assert_eq!(recovery["baseline"], FIXTURE);
    assert!(
        recovery["draft"]["source"]
            .as_str()
            .unwrap()
            .contains("My local draft.")
    );
    assert_eq!(session.model().view(), &selected);
    assert_eq!(session.model().document().source(), external);
    assert!(!session.dirty());
}

#[test]
fn recovery_lease_prevents_a_second_frontend_from_owning_the_same_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scene.recite");
    fs::write(&path, FIXTURE).unwrap();
    let session = open(path.clone()).unwrap();
    assert!(matches!(
        DocumentSession::open(
            path.clone(),
            DocumentKey::new("scene.recite").unwrap(),
            context(FIXTURE).unwrap()
        ),
        Err(FileError::RecoveryInUse)
    ));
    drop(session);
    drop(open(path).unwrap());
}

#[test]
fn editor_handoff_preserves_an_unapplied_draft_without_requiring_it_to_validate() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scene.recite");
    fs::write(&path, FIXTURE).unwrap();
    let mut session = open(path.clone()).unwrap();
    session
        .edit(|model| {
            model.set_draft(String::new());
            Ok(())
        })
        .unwrap();
    session.prepare_external_edit().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), FIXTURE);
    assert!(session.model().has_draft());
    drop(session);
    let restored = open(path).unwrap();
    assert!(restored.model().has_draft());
    assert_eq!(restored.model().draft(), "");
}

#[test]
fn malformed_external_source_remains_intact_and_enters_repair_state() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scene.recite");
    fs::write(&path, FIXTURE).unwrap();
    let mut session = open(path.clone()).unwrap();
    let malformed = ":: scene default\n->\n";
    fs::write(&path, malformed).unwrap();
    session.reload(context(malformed).unwrap()).unwrap();
    assert_eq!(session.model().view(), &View::Source);
    assert_eq!(session.model().document().source(), malformed);
    assert!(!session.model().document().diagnostic_snapshot().is_empty());
    assert_eq!(fs::read_to_string(path).unwrap(), malformed);
}

#[test]
fn reload_refuses_excluded_document_and_preserves_the_local_draft() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scene.recite");
    fs::write(&path, FIXTURE).unwrap();
    let mut session = open(path).unwrap();
    session
        .edit(|model| {
            model.set_draft("Keep me.".into());
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        session.reload(ProjectContext {
            documents: vec![],
            schema: None
        }),
        Err(FileError::SessionExcluded(_))
    ));
    assert_eq!(session.model().draft(), "Keep me.");
    session.edit(Workbench::apply).unwrap();
    assert!(session.model().document().source().contains("Keep me."));
}
