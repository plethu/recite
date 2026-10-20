use recite_writer_model::{View, Workbench};
use recite_writer_session::{FileError, ProjectSession};
use std::{fs, path::Path};

const FIRST: &str = ":: first default\n> first@11111111111111111111\n  First paragraph.\n-> second.recite::second\n";
const SECOND: &str = ":: second\n> second@22222222222222222222\n  Second paragraph.\n-> END\n";
fn fixture() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version = 1\n",
    )?;
    fs::write(directory.path().join("first.recite"), FIRST)?;
    fs::write(directory.path().join("second.recite"), SECOND)?;
    Ok(directory)
}
fn edit(project: &mut ProjectSession, text: &str, apply: bool) -> Result<(), FileError> {
    project.current_mut().edit(|model| {
        model.set_draft(text.into());
        if apply {
            model.apply()?;
        }
        Ok(())
    })
}

#[test]
fn switching_keeps_independent_drafts_selections_applied_history_and_recovery_leases() {
    let directory = fixture().unwrap();
    let mut project = ProjectSession::open(directory.path(), None).unwrap();
    let first = project.current().path().to_owned();
    let second = directory.path().join("second.recite");
    let selected = project.current().model().view().clone();
    edit(&mut project, "Accepted first.", true).unwrap();
    edit(&mut project, "Unapplied first.", false).unwrap();
    project.switch(&second).unwrap();
    edit(&mut project, "Accepted second.", true).unwrap();
    project.switch(&first).unwrap();
    assert_eq!(project.current().model().draft(), "Unapplied first.");
    assert_eq!(project.current().model().view(), &selected);
    project
        .current_mut()
        .edit(|model| {
            model.discard();
            model.undo()
        })
        .unwrap();
    assert_eq!(project.current().model().draft(), "First paragraph.");
    project.current_mut().edit(Workbench::redo).unwrap();
    assert_eq!(project.current().model().draft(), "Accepted first.");
    assert!(matches!(
        ProjectSession::open(directory.path(), Some(Path::new("second.recite"))),
        Err(FileError::RecoveryInUse)
    ));
    project.checkpoint().unwrap();
    drop(project);
    let mut project = ProjectSession::open(directory.path(), None).unwrap();
    project.switch(&second).unwrap();
    assert_eq!(project.current().model().draft(), "Accepted second.");
    project.save_all().unwrap();
    assert!(!project.dirty());
    assert!(
        fs::read_to_string(first)
            .unwrap()
            .contains("Accepted first.")
    );
    assert!(
        fs::read_to_string(second)
            .unwrap()
            .contains("Accepted second.")
    );
}

#[test]
fn accepted_sibling_overlays_drive_diagnostics_and_unapplied_drafts_do_not() {
    let directory = fixture().unwrap();
    let mut project = ProjectSession::open(directory.path(), None).unwrap();
    let first = project.current().path().to_owned();
    let second = directory.path().join("second.recite");
    project.switch(&second).unwrap();
    project
        .current_mut()
        .edit(|model| model.select(View::Source))
        .unwrap();
    edit(
        &mut project,
        &SECOND.replace(":: second", ":: renamed"),
        false,
    )
    .unwrap();
    project.switch(&first).unwrap();
    assert!(
        project
            .current()
            .model()
            .document()
            .diagnostics()
            .is_empty()
    );
    project.switch(&second).unwrap();
    project.current_mut().edit(Workbench::apply).unwrap();
    project.switch(&first).unwrap();
    assert!(
        !project
            .current()
            .model()
            .document()
            .diagnostics()
            .is_empty()
    );
    assert_eq!(fs::read_to_string(second).unwrap(), SECOND);
}

#[test]
fn failed_open_and_excluded_retained_file_leave_drafts_and_active_ownership_intact() {
    let directory = fixture().unwrap();
    let mut project = ProjectSession::open(directory.path(), None).unwrap();
    let first = project.current().path().to_owned();
    let second = directory.path().join("second.recite");
    let other = recite_writer_session::recovery::RecoveryStore::open(&second).unwrap();
    edit(&mut project, "Keep this draft.", false).unwrap();
    assert!(matches!(
        project.switch(&second),
        Err(FileError::RecoveryInUse)
    ));
    assert_eq!(project.current().path(), first);
    assert_eq!(project.current().model().draft(), "Keep this draft.");
    drop(other);
    project.switch(&second).unwrap();
    fs::remove_file(&first).unwrap();
    assert!(
        matches!(project.reload_current(), Err(FileError::SessionExcluded(path)) if path == first)
    );
    project.switch(&first).unwrap();
    assert_eq!(project.current().model().draft(), "Keep this draft.");
    assert!(matches!(
        project.switch(&directory.path().join("missing.recite")),
        Err(FileError::Selection)
    ));
}

#[test]
fn save_all_stops_on_conflict_without_overwriting_disk_or_losing_inactive_draft() {
    let directory = fixture().unwrap();
    let mut project = ProjectSession::open(directory.path(), None).unwrap();
    let first = project.current().path().to_owned();
    let second = directory.path().join("second.recite");
    edit(&mut project, "Local first.", false).unwrap();
    project.switch(&second).unwrap();
    edit(&mut project, "Local second.", false).unwrap();
    let external = format!("{FIRST}\n# External change\n");
    fs::write(&first, &external).unwrap();
    assert!(matches!(project.save_all(), Err(FileError::Saving { path, .. }) if path == first));
    assert!(project.dirty());
    assert_eq!(fs::read_to_string(&first).unwrap(), external);
    assert!(
        fs::read_to_string(second)
            .unwrap()
            .contains("Local second.")
    );
    project.switch(&first).unwrap();
    assert!(
        project
            .current()
            .model()
            .document()
            .source()
            .contains("Local first.")
    );
    assert!(project.reload_current().unwrap().unwrap().exists());
    assert!(!project.dirty());
}

#[test]
fn requested_document_does_not_acquire_an_unrelated_first_document_lease() {
    let directory = fixture().unwrap();
    let first = directory.path().join("first.recite");
    let lease = recite_writer_session::recovery::RecoveryStore::open(&first).unwrap();
    let mut project =
        ProjectSession::open(directory.path(), Some(Path::new("second.recite"))).unwrap();
    edit(&mut project, "Keep requested document.", false).unwrap();
    project.checkpoint().unwrap();
    let sidecar = directory
        .path()
        .join("second.recite.recite-editor-recovery.json");
    let recovery = fs::read(&sidecar).unwrap();
    assert!(matches!(
        ProjectSession::open(directory.path(), Some(Path::new("second.recite"))),
        Err(FileError::RecoveryInUse)
    ));
    assert_eq!(fs::read(&sidecar).unwrap(), recovery);
    assert_eq!(
        project.current().model().draft(),
        "Keep requested document."
    );
    assert!(matches!(
        project.switch(&first),
        Err(FileError::RecoveryInUse)
    ));
    drop(lease);
    project.switch(&first).unwrap();
}
