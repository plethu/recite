use super::*;
use recite_writer_model::FIXTURE;
use std::fs;
#[path = "tests/declarations/tests.rs"]
mod declarations;

fn project() -> (tempfile::TempDir, ProjectFiles, Workbench) {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n",
    )
    .unwrap();
    fs::write(dir.path().join("first.recite"), FIXTURE).unwrap();
    fs::write(
        dir.path().join("second.recite"),
        ":: second\n> text@22222222222222222222\n  Second.\n-> END\n",
    )
    .unwrap();
    let mut files = ProjectFiles::open(dir.path()).unwrap();
    let model = files.workbench().unwrap();
    (dir, files, model)
}
fn finish(
    sequence: &mut SaveSequence,
    files: &mut ProjectFiles,
    model: &mut Workbench,
) -> Result<(), FileError> {
    loop {
        match sequence.poll(files, model) {
            Progress::Work(work) => sequence.accept(files, model, work.execute()),
            Progress::Finished(result) => return result,
            Progress::Waiting => panic!("request was not accepted"),
        }
    }
}
#[test]
fn newer_typing_survives_a_save_and_successful_receipt_is_always_reconciled() {
    let (dir, mut files, mut model) = project();
    model.set_draft("Captured accepted edit.".into());
    model.apply().unwrap();
    let accepted = model.document().source().to_owned();
    let mut sequence = SaveSequence::accepted(&mut files, &model, Scope::Current);
    let Progress::Work(work) = sequence.poll(&mut files, &mut model) else {
        panic!()
    };
    assert!(files.saving);
    assert!(matches!(
        files.require_idle_save(),
        Err(FileError::SavePending)
    ));
    assert!(matches!(
        sequence.poll(&mut files, &mut model),
        Progress::Waiting
    ));
    let receipt = work.execute();
    model.set_draft("Typed while the save was running.".into());
    sequence.accept(&mut files, &mut model, receipt);
    // A cancellation request can close progress UI; it cannot discard this write.
    finish(&mut sequence, &mut files, &mut model).unwrap();
    assert!(!files.saving);
    assert_eq!(fs::read_to_string(&files.current).unwrap(), accepted);
    assert_eq!(model.draft(), "Typed while the save was running.");
    assert!(model.has_draft());
    assert!(files.has_recovery());
    drop(files);
    let mut reopened = ProjectFiles::open(dir.path()).unwrap();
    assert_eq!(reopened.workbench().unwrap().draft(), model.draft());
}
#[test]
fn second_file_conflict_preserves_first_publication_and_retained_edits() {
    let (dir, mut files, mut model) = project();
    let first = files.current.clone();
    let second = dir.path().join("second.recite");
    files.switch(&mut model, &second, None, |_| Ok(())).unwrap();
    model.set_draft("Retained local edit.".into());
    files.switch(&mut model, &first, None, |_| Ok(())).unwrap();
    model.set_draft("First saved edit.".into());
    model.apply().unwrap();
    let external = ":: external\n-> END\n";
    fs::write(&second, external).unwrap();
    let mut sequence = SaveSequence::accepted(&mut files, &model, Scope::All);
    assert!(
        matches!(finish(&mut sequence, &mut files, &mut model), Err(FileError::Saving {path, ..}) if path == second)
    );
    assert!(!files.saving);
    assert!(
        fs::read_to_string(&first)
            .unwrap()
            .contains("First saved edit.")
    );
    assert!(!files.dirty(model.document().source()));
    assert_eq!(fs::read_to_string(&second).unwrap(), external);
    files.switch(&mut model, &second, None, |_| Ok(())).unwrap();
    assert!(model.document().source().contains("Retained local edit."));
    assert!(files.dirty(model.document().source()));
}
#[test]
fn recovery_failure_after_replacement_advances_baseline_and_can_retry() {
    let (_dir, mut files, mut model) = project();
    model.set_draft("Saved before recovery failed.".into());
    model.apply().unwrap();
    files.checkpoint(&model).unwrap();
    let sidecar = files
        .current
        .with_file_name("first.recite.recite-editor-recovery.json");
    fs::remove_file(&sidecar).unwrap();
    fs::create_dir(&sidecar).unwrap();
    let mut sequence = SaveSequence::accepted(&mut files, &model, Scope::Current);
    assert!(finish(&mut sequence, &mut files, &mut model).is_err());
    assert!(!files.saving);
    assert_eq!(
        fs::read_to_string(&files.current).unwrap(),
        model.document().source()
    );
    fs::remove_dir(&sidecar).unwrap();
    files.save_current(&mut model).unwrap();
    assert!(!files.dirty(model.document().source()));
}
