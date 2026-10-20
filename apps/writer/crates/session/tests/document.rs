use recite_writer_model::{FIXTURE, View, Workbench};
use recite_writer_session::{FileError, ProjectSession};
use std::{fs, path::PathBuf};

fn open(path: PathBuf) -> Result<ProjectSession, Box<dyn std::error::Error>> {
    let root = path.parent().unwrap();
    let manifest = root.join("recite.project.toml");
    if !manifest.exists() {
        fs::write(manifest, "format_version = 1\n")?;
    }
    Ok(ProjectSession::open(root, Some(&path))?)
}

#[test]
fn queued_unicode_draft_survives_close_and_save_preserves_stable_identity() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scene.recite");
    fs::write(&path, FIXTURE).unwrap();
    let mut session = open(path.clone()).unwrap();
    let selected = session.current().model().view().clone();
    session
        .current_mut()
        .edit(|model| {
            model.set_draft("Café 日本語 مرحبا".into());
            Ok(())
        })
        .unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), FIXTURE);
    drop(session);
    let mut restored = open(path.clone()).unwrap();
    assert_eq!(restored.current().model().draft(), "Café 日本語 مرحبا");
    assert_eq!(restored.current().model().view(), &selected);
    restored.current_mut().save().unwrap();
    assert!(!restored.dirty());
    assert_eq!(restored.current().model().view(), &selected);
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
    let selected = session.current().model().view().clone();
    session
        .current_mut()
        .edit(|model| {
            model.set_draft("My local draft.".into());
            Ok(())
        })
        .unwrap();
    let external = format!("{FIXTURE}\n# An external edit\n");
    fs::write(&path, &external).unwrap();
    assert!(matches!(
        session.current_mut().save(),
        Err(FileError::Conflict)
    ));
    assert_eq!(fs::read_to_string(&path).unwrap(), external);
    let copy = session.reload_current().unwrap().unwrap();
    let recovery: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(copy).unwrap()).unwrap();
    assert_eq!(recovery["baseline"], FIXTURE);
    assert!(
        recovery["draft"]["source"]
            .as_str()
            .unwrap()
            .contains("My local draft.")
    );
    assert_eq!(session.current().model().view(), &selected);
    assert_eq!(session.current().model().document().source(), external);
    assert!(!session.dirty());
}

#[test]
fn recovery_lease_prevents_a_second_frontend_from_owning_the_same_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scene.recite");
    fs::write(&path, FIXTURE).unwrap();
    let session = open(path.clone()).unwrap();
    assert!(matches!(
        ProjectSession::open(directory.path(), Some(&path)),
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
        .current_mut()
        .edit(|model| {
            model.set_draft(String::new());
            Ok(())
        })
        .unwrap();
    session.current_mut().prepare_external_edit().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), FIXTURE);
    assert!(session.current().model().has_draft());
    drop(session);
    let restored = open(path).unwrap();
    assert!(restored.current().model().has_draft());
    assert_eq!(restored.current().model().draft(), "");
}

#[test]
fn malformed_external_source_remains_intact_and_enters_repair_state() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scene.recite");
    fs::write(&path, FIXTURE).unwrap();
    let mut session = open(path.clone()).unwrap();
    let malformed = ":: scene default\n->\n";
    fs::write(&path, malformed).unwrap();
    session.reload_current().unwrap();
    assert_eq!(session.current().model().view(), &View::Source);
    assert_eq!(session.current().model().document().source(), malformed);
    assert!(
        !session
            .current()
            .model()
            .document()
            .diagnostic_snapshot()
            .is_empty()
    );
    assert_eq!(fs::read_to_string(path).unwrap(), malformed);
}

#[test]
fn reload_refuses_excluded_document_and_preserves_the_local_draft() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scene.recite");
    fs::write(&path, FIXTURE).unwrap();
    let mut session = open(path).unwrap();
    session
        .current_mut()
        .edit(|model| {
            model.set_draft("Keep me.".into());
            Ok(())
        })
        .unwrap();
    fs::rename(
        directory.path().join("scene.recite"),
        directory.path().join("scene.txt"),
    )
    .unwrap();
    assert!(matches!(
        session.reload_current(),
        Err(FileError::SessionExcluded(_))
    ));
    assert_eq!(session.current().model().draft(), "Keep me.");
    session.current_mut().edit(Workbench::apply).unwrap();
    assert!(
        session
            .current()
            .model()
            .document()
            .source()
            .contains("Keep me.")
    );
}
