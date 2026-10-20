use super::*;
use crate::recovery::{Recovery, RecoveryStore};
use recite_writer_model::View;
use std::{fs, io};
const SOURCE: &str =
    ":: start default\n> line@11111111111111111111\n  Hello.\n-> old\n\n:: old\n-> END\n";
fn fixture() -> (tempfile::TempDir, ProjectFiles, Workbench) {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("recite.project.toml"),
        "format_version=1\n",
    )
    .unwrap();
    fs::write(root.path().join("a.recite"), SOURCE).unwrap();
    fs::write(root.path().join("z.recite"), ":: other\n-> a.recite::old\n").unwrap();
    let mut files = ProjectFiles::open(root.path()).unwrap();
    let model = files.workbench().unwrap();
    (root, files, model)
}
#[test]
fn cold_recovered_changes_are_retained_and_refuse_the_reviewed_rename() {
    let (root, mut files, mut model) = fixture();
    files.review_rename(&mut model, "old", "new").unwrap();
    let path = root.path().join("z.recite");
    let baseline = read_regular(&path).unwrap();
    let changed = baseline.replace(":: other", ":: recovered");
    let recovered = Workbench::open("z.recite", &changed).unwrap();
    let mut store = RecoveryStore::open(&path).unwrap();
    store
        .persist(Some(Recovery::new(baseline.into(), recovered.recovery())))
        .unwrap();
    drop(store);
    assert!(matches!(
        files.apply_rename(&mut model),
        Err(FileError::RecoveredRenameInput(_))
    ));
    assert_eq!(model.document().source(), SOURCE);
    assert_eq!(files.retained[&path].model.document().source(), changed);
    assert!(!files.project_edit_recovery_pending());
    assert_eq!(files.current, root.path().join("a.recite"));
}
#[test]
fn final_grouped_cleanup_sync_failure_keeps_edit_navigation_and_save_excluded_until_retry() {
    let (root, mut files, mut model) = fixture();
    model
        .select(View::Passage("11111111111111111111".into()))
        .unwrap();
    files.review_rename(&mut model, "old", "new").unwrap();
    let blocked = root.path().join("a.recite.recite-editor-recovery.json");
    fs::create_dir(&blocked).unwrap();
    assert!(files.apply_rename(&mut model).is_err());
    assert!(files.project_edit_recovery_pending());
    assert!(model.document().source().contains(":: new"));
    fs::remove_dir(&blocked).unwrap();
    assert!(
        files
            .finish_project_edit_with_sync(&model, |_| Err(io::Error::other("final sync failed")))
            .is_err()
    );
    assert!(files.manifest.pending().is_empty());
    assert!(files.project_edit_recovery_pending());
    assert!(matches!(
        files.require_project_edit_ready(),
        Err(FileError::ProjectEditRecoveryPending)
    ));
    assert!(files.require_navigation_ready().is_err());
    assert!(files.require_idle_save().is_err());
    assert!(files.checkpoint(&model).is_err());
    files.finish_project_edit(&model).unwrap();
    assert!(!files.project_edit_recovery_pending());
    assert!(model.document().source().contains(":: new"));
}
