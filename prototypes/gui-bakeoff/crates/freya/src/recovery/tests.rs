use super::*;
use recite_bakeoff_authoring::Workbench;

#[test]
fn snapshot_survives_reopening_and_lock_prevents_two_owners()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("scene.recite");
    let mut workbench = Workbench::new(recite_bakeoff_authoring::FIXTURE)?;
    workbench.set_draft("A recovered question.".into());
    let recovery = Recovery::new(
        recite_bakeoff_authoring::FIXTURE.into(),
        workbench.recovery(),
    );
    let mut store = RecoveryStore::open(&source)?;
    store.persist(Some(recovery.clone()))?;
    assert!(matches!(
        RecoveryStore::open(&source),
        Err(FileError::RecoveryInUse)
    ));
    drop(store);
    let mut reopened = RecoveryStore::open(&source)?;
    assert_eq!(reopened.snapshot(), Some(&recovery));
    reopened.persist(None)?;
    drop(reopened);
    assert!(RecoveryStore::open(&source)?.snapshot().is_none());
    Ok(())
}

#[test]
fn corrupt_and_future_recovery_are_left_untouched() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("scene.recite");
    let path = sidecar(&source, ".recite-editor-recovery.json");
    fs::write(&path, "broken json")?;
    assert!(matches!(
        RecoveryStore::open(&source),
        Err(FileError::Recovery(_))
    ));
    assert_eq!(fs::read_to_string(&path)?, "broken json");
    let mut recovery = Recovery::new(
        String::new(),
        Workbench::new(recite_bakeoff_authoring::FIXTURE)?.recovery(),
    );
    recovery.version = 2;
    let text = serde_json::to_string(&recovery)?;
    fs::write(&path, &text)?;
    assert!(matches!(
        RecoveryStore::open(&source),
        Err(FileError::RecoveryVersion)
    ));
    assert_eq!(fs::read_to_string(&path)?, text);
    Ok(())
}
