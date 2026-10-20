use super::super::tests::{SOURCE, fixture};
use super::*;
use recite_compiler::authoring::CancellationToken;
use std::fs;

#[test]
fn merge_export_materializes_pending_prose_without_applying_it_to_the_live_session() {
    let (_directory, mut files, mut model) = fixture();
    model.set_draft("Pending local text.".into());
    let before = model.recovery();
    let stamp = model.navigation_stamp();
    let disk = SOURCE.replace("Original.", "Inspected external text.");
    fs::write(&files.current, &disk).unwrap();
    files.compare_external(&model).unwrap();
    let artifact = files
        .export_external_merge(&model, &CancellationToken::new())
        .unwrap();
    assert!(stamp.is_current(&model));
    assert_eq!(model.recovery(), before);
    assert!(
        fs::read_to_string(&artifact.source)
            .unwrap()
            .contains("Pending local text.")
    );
    assert_eq!(fs::read_to_string(&files.current).unwrap(), disk);
    let recovery: Recovery =
        serde_json::from_str(&fs::read_to_string(&artifact.recovery).unwrap()).unwrap();
    assert_eq!(recovery.draft, before);
    assert_eq!(recovery.baseline.as_ref(), SOURCE);
    assert_eq!(
        files.external.as_ref().unwrap().artifact().unwrap().source,
        artifact.source
    );

    let merged = SOURCE.replace("Original.", "Reviewed reconciliation.");
    fs::write(&artifact.source, &merged).unwrap();
    let resolution = files
        .import_external_merge(&mut model, &artifact.source, &CancellationToken::new())
        .unwrap();
    assert!(resolution.recovery_error.is_none());
    assert_eq!(model.document().source(), merged);
    assert_eq!(model.draft(), "Reviewed reconciliation.");
    assert_eq!(files.saved.as_ref(), disk);
    assert_eq!(fs::read_to_string(&files.current).unwrap(), disk);
    model.undo().unwrap();
    assert_eq!(model.document().source(), SOURCE);
    model.redo().unwrap();
    files.save_current(&mut model).unwrap();
    assert_eq!(fs::read_to_string(&files.current).unwrap(), merged);
}

#[test]
fn invalid_import_and_a_new_disk_version_keep_the_local_draft_and_existing_artifact() {
    let (_directory, mut files, mut model) = fixture();
    model.set_draft("Still pending.".into());
    let before = model.recovery();
    let disk = SOURCE.replace("Original.", "External.");
    fs::write(&files.current, &disk).unwrap();
    files.compare_external(&model).unwrap();
    let artifact = files
        .export_external_merge(&model, &CancellationToken::new())
        .unwrap();
    fs::write(&artifact.source, ":: start default\n? invalid").unwrap();
    assert!(
        files
            .import_external_merge(&mut model, &artifact.source, &CancellationToken::new())
            .is_err()
    );
    assert_eq!(model.recovery(), before);
    assert_eq!(files.saved.as_ref(), SOURCE);
    fs::write(&artifact.source, SOURCE.replace("Original.", "Merged.")).unwrap();
    fs::write(&files.current, disk.replace("External.", "Newer external.")).unwrap();
    assert!(matches!(
        files.import_external_merge(&mut model, &artifact.source, &CancellationToken::new()),
        Err(FileError::Conflict)
    ));
    assert_eq!(model.recovery(), before);
    assert_eq!(files.saved.as_ref(), SOURCE);
    assert!(artifact.source.is_file() && artifact.recovery.is_file());
}
