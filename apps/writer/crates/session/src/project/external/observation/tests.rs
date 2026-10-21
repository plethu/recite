use super::super::tests::{SOURCE, fixture};
use super::*;
use std::fs;

#[test]
fn captured_observation_keeps_draft_edits_independent_and_reports_atomic_replacement_and_deletion()
{
    let (directory, files, mut model) = fixture();
    let request = files.external_check(&files.current).unwrap();
    model.set_draft("A newer field draft.".into());
    let draft = model.recovery();
    let replacement = directory.path().join("replacement");
    fs::write(&replacement, SOURCE.replace("Original.", "External.")).unwrap();
    fs::rename(replacement, &files.current).unwrap();
    let observation = request.observe();
    assert!(observation.changed() && observation.is_current(&files));
    assert_eq!(model.recovery(), draft);
    let removed = files.external_check(&files.current).unwrap();
    fs::remove_file(&files.current).unwrap();
    assert!(removed.observe().changed());
}

#[test]
fn source_checks_share_saved_inputs_and_the_receipt_has_no_strong_source_owner_or_recovery_lease() {
    let (directory, files, model) = fixture();
    let strong = Arc::strong_count(&files.saved);
    let context = Arc::strong_count(&files.context);
    let requests = files.external_checks(&Affected::AllOpen);
    assert_eq!(requests.len(), 1);
    assert_eq!(Arc::strong_count(&files.saved), strong + 1);
    assert_eq!(Arc::strong_count(&files.context), context);
    let observation = requests.into_iter().next().unwrap().observe();
    assert!(!observation.changed());
    assert_eq!(Arc::strong_count(&files.saved), strong);
    drop(model);
    drop(files);
    let reopened = ProjectFiles::open(directory.path()).unwrap();
    assert!(!observation.is_current(&reopened));
}

#[test]
fn completing_our_own_save_invalidates_an_observation_captured_against_the_previous_baseline() {
    let (_directory, mut files, mut model) = fixture();
    let observation = files.external_check(&files.current).unwrap().observe();
    model.set_draft("Saved here.".into());
    files.save_current(&mut model).unwrap();
    assert!(!observation.is_current(&files));
    let current = files.external_check(&files.current).unwrap().observe();
    assert!(current.is_current(&files) && !current.changed());
}

#[test]
fn a_rescan_captures_each_open_document_once_including_retained_sources() {
    use crate::ProjectSession;
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version=1\n",
    )
    .unwrap();
    let first = directory.path().join("scene.recite");
    let second = directory.path().join("z.recite");
    fs::write(&first, SOURCE).unwrap();
    fs::write(
        &second,
        SOURCE
            .replace(":: start default", ":: other")
            .replace("11111111111111111111", "33333333333333333333"),
    )
    .unwrap();
    let mut project = ProjectSession::open(directory.path(), None).unwrap();
    project.switch(&second).unwrap();
    fs::write(&first, SOURCE.replace("Original.", "First external edit.")).unwrap();
    fs::write(
        &second,
        SOURCE.replace("Original.", "Second external edit."),
    )
    .unwrap();
    let observations: Vec<_> = project
        .files()
        .external_checks(&Affected::AllOpen)
        .into_iter()
        .map(ExternalCheck::observe)
        .collect();
    assert_eq!(observations.len(), 2);
    assert!(
        observations
            .iter()
            .all(|observation| observation.changed() && observation.is_current(project.files()))
    );
}
