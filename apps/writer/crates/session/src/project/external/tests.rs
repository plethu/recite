use super::*;
use recite_writer_model::View;
use std::fs;
#[path = "faults/tests.rs"]
mod faults;
#[path = "navigation/tests.rs"]
mod navigation;

pub(super) const SOURCE: &str = ":: start default\n> @ffee6p8\n  Original.\n-> END\n";
const LINE: &str = "ffee6p8";
pub(super) fn fixture() -> (tempfile::TempDir, ProjectFiles, Workbench) {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version=1\n",
    )
    .unwrap();
    fs::write(directory.path().join("scene.recite"), SOURCE).unwrap();
    let mut files = ProjectFiles::open(directory.path()).unwrap();
    let mut model = files.workbench().unwrap();
    model.select(View::Passage(LINE.into())).unwrap();
    (directory, files, model)
}

#[test]
fn comparison_observes_accepted_source_and_keeps_pending_prose_selection_and_history() {
    let (_directory, mut files, mut model) = fixture();
    model.set_draft("Applied.".into());
    model.apply().unwrap();
    model.set_draft("Unapplied.".into());
    let before = model.recovery();
    let source = model.document().source_snapshot();
    let stamp = model.navigation_stamp();
    let disk = SOURCE.replace("Original.", "External.");
    fs::write(&files.current, &disk).unwrap();
    files.compare_external(&model).unwrap();
    let comparison = files.external.as_ref().unwrap();
    assert!(stamp.is_current(&model));
    assert!(Arc::ptr_eq(&source, &model.document().source_snapshot()));
    assert!(model.document().can_undo());
    assert_eq!(model.recovery(), before);
    assert!(comparison.writer_source().contains("Applied."));
    assert!(!comparison.writer_source().contains("Unapplied."));
    assert_eq!(
        comparison
            .pending_draft()
            .unwrap()
            .draft_snapshot()
            .as_str(),
        "Unapplied."
    );
    assert_eq!(comparison.disk_source(), disk);
}

#[test]
fn keeping_writer_retains_the_exact_field_and_document_undo_until_explicit_save() {
    let (_directory, mut files, mut model) = fixture();
    model.set_draft("Applied.".into());
    model.apply().unwrap();
    model.set_draft("Unapplied.".into());
    let before = model.recovery();
    let disk = SOURCE.replace("Original.", "External.");
    fs::write(&files.current, &disk).unwrap();
    files.compare_external(&model).unwrap();
    let archive = files
        .resolve_external(&mut model, ExternalDecision::KeepWriter)
        .unwrap()
        .archive;
    assert_eq!(model.recovery(), before);
    assert!(model.document().can_undo());
    assert_eq!(files.saved.as_ref(), disk);
    assert_eq!(fs::read_to_string(&files.current).unwrap(), disk);
    let backup: Recovery = serde_json::from_str(&fs::read_to_string(archive).unwrap()).unwrap();
    assert_eq!(backup.draft, before);
    assert_eq!(backup.baseline.as_ref(), SOURCE);
    model.discard();
    model.undo().unwrap();
    assert_eq!(model.document().source(), SOURCE);
    model.redo().unwrap();
    model.set_draft("Unapplied.".into());
    files.save_current(&mut model).unwrap();
    assert!(
        fs::read_to_string(&files.current)
            .unwrap()
            .contains("Unapplied.")
    );
}

#[test]
fn newer_local_field_or_selection_and_newer_disk_refuse_the_inspected_decision() {
    for decision in [
        ExternalDecision::KeepWriter,
        ExternalDecision::UseInspectedDisk,
    ] {
        for change in ["draft", "selection", "disk"] {
            let (_directory, mut files, mut model) = fixture();
            let disk = SOURCE.replace("Original.", "External.");
            fs::write(&files.current, &disk).unwrap();
            files.compare_external(&model).unwrap();
            match change {
                "draft" => model.set_draft("Newer local typing.".into()),
                "selection" => model.select(View::Block("start".into())).unwrap(),
                _ => fs::write(&files.current, disk.replace("External.", "Newer disk.")).unwrap(),
            }
            let before = model.recovery();
            assert!(matches!(
                files.resolve_external(&mut model, decision),
                Err(FileError::Conflict)
            ));
            assert_eq!(model.recovery(), before);
            assert_eq!(files.saved.as_ref(), SOURCE);
            assert!(files.external.is_some());
        }
    }
}

#[test]
fn admitted_keep_with_failed_recovery_keeps_the_model_and_reports_the_adopted_baseline() {
    let (directory, mut files, mut model) = fixture();
    model.set_draft("Pending prose.".into());
    let before = model.recovery();
    let disk = SOURCE.replace("Original.", "External.");
    fs::write(&files.current, &disk).unwrap();
    files.compare_external(&model).unwrap();
    let recovery = directory
        .path()
        .join("scene.recite.recite-editor-recovery.json");
    fs::create_dir(&recovery).unwrap();
    let resolution = files
        .resolve_external(&mut model, ExternalDecision::KeepWriter)
        .unwrap();
    assert!(resolution.recovery_error.is_some());
    assert_eq!(model.recovery(), before);
    assert_eq!(files.saved.as_ref(), disk);
    assert!(files.external.is_none());
    assert!(resolution.archive.is_file());
    fs::remove_dir(recovery).unwrap();
    files.checkpoint(&model).unwrap();
    assert_eq!(model.recovery(), before);
    assert_eq!(files.saved.as_ref(), disk);
}

#[test]
fn adopting_inspected_disk_archives_pending_prose_and_remains_one_undo_step() {
    let (_directory, mut files, mut model) = fixture();
    model.set_draft("Applied before inspection.".into());
    model.apply().unwrap();
    let accepted = model.document().source().to_owned();
    model.set_draft("Pending before adoption.".into());
    let before = model.recovery();
    let disk = SOURCE.replace("Original.", "Inspected disk.");
    fs::write(&files.current, &disk).unwrap();
    files.compare_external(&model).unwrap();
    let archive = files
        .resolve_external(&mut model, ExternalDecision::UseInspectedDisk)
        .unwrap()
        .archive;
    assert_eq!(model.view(), &View::Passage(LINE.into()));
    assert_eq!(model.draft(), "Inspected disk.");
    assert_eq!(model.document().source(), disk);
    assert_eq!(files.saved.as_ref(), disk);
    let backup: Recovery = serde_json::from_str(&fs::read_to_string(archive).unwrap()).unwrap();
    assert_eq!(backup.draft, before);
    model.undo().unwrap();
    assert_eq!(model.document().source(), accepted);
    assert!(files.dirty(model.document().source()));
    model.undo().unwrap();
    assert_eq!(model.document().source(), SOURCE);
    model.redo().unwrap();
    model.redo().unwrap();
    assert_eq!(model.document().source(), disk);
    assert!(!files.dirty(model.document().source()));
}

#[test]
fn adopting_disk_clears_the_draft_and_recovery_and_allows_navigation()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n[project]\n",
    )?;
    std::fs::write(
        dir.path().join("scene.recite"),
        recite_writer_model::FIXTURE,
    )?;
    let mut files = ProjectFiles::open(dir.path())?;
    let mut model = files.workbench()?;
    model.select(View::Source)?;
    let draft = format!("{}\n# local draft\n", model.document().source());
    model.set_draft(draft.clone());
    files.checkpoint(&model)?;
    let disk = recite_writer_model::FIXTURE.replace("Would you tell me", "Could you tell me");
    std::fs::write(&files.current, &disk)?;
    files.compare_external(&model)?;
    let copy = files
        .resolve_external(&mut model, ExternalDecision::UseInspectedDisk)?
        .archive;

    assert_eq!(model.document().source(), disk);
    assert_eq!(model.draft(), disk);
    assert!(!model.has_draft());
    assert!(!files.dirty(model.document().source()));
    assert!(!files.has_recovery());
    assert!(
        !files
            .external_check(&files.current)
            .unwrap()
            .observe()
            .changed()
    );
    assert_eq!(std::fs::read_to_string(&files.current)?, disk);
    let exported: crate::recovery::Recovery =
        serde_json::from_str(&std::fs::read_to_string(copy)?)?;
    let mut recovered = recite_writer_model::Workbench::new(recite_writer_model::FIXTURE)?;
    exported.draft.restore(&mut recovered)?;
    assert_eq!(recovered.draft(), draft);
    let passage = model.document().passages()?[0].id.clone();
    model.select(View::Passage(passage))?;
    Ok(())
}
