use super::*;
use recite_writer_model::Workbench;

#[test]
fn queued_checkpoints_flush_the_latest_draft_and_clear_without_resurrection()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("scene.recite");
    let mut workbench = Workbench::new(recite_writer_model::FIXTURE)?;
    let mut store = RecoveryStore::open(&source)?;
    for index in 0..30 {
        workbench.set_draft(format!("Draft {index}"));
        store.queue(Some(Recovery::new(
            recite_writer_model::FIXTURE.into(),
            workbench.recovery(),
        )))?;
    }
    let latest = Recovery::new(recite_writer_model::FIXTURE.into(), workbench.recovery());
    store.persist(Some(latest.clone()))?;
    drop(store);
    let mut reopened = RecoveryStore::open(&source)?;
    assert_eq!(reopened.snapshot(), Some(&latest));
    reopened.persist(None)?;
    drop(reopened);
    assert!(RecoveryStore::open(&source)?.snapshot().is_none());
    Ok(())
}

#[test]
fn background_failure_is_reported_and_explicit_persistence_can_retry()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let child = directory.path().join("content");
    fs::create_dir(&child)?;
    let mut store = RecoveryStore::open(&child.join("scene.recite"))?;
    let snapshot = Some(Recovery::new(
        recite_writer_model::FIXTURE.into(),
        Workbench::new(recite_writer_model::FIXTURE)?.recovery(),
    ));
    fs::remove_dir_all(&child)?;
    assert!(matches!(
        store.persist(snapshot.clone()),
        Err(FileError::BackgroundRecovery(_))
    ));
    assert!(store.error().is_some());
    fs::create_dir(&child)?;
    store.persist(snapshot)?;
    assert!(store.error().is_none());
    Ok(())
}

#[test]
fn snapshot_survives_reopening_and_lock_prevents_two_owners()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("scene.recite");
    let mut workbench = Workbench::new(recite_writer_model::FIXTURE)?;
    workbench.set_draft("A recovered question.".into());
    let recovery = Recovery::new(recite_writer_model::FIXTURE.into(), workbench.recovery());
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
        String::new().into(),
        Workbench::new(recite_writer_model::FIXTURE)?.recovery(),
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

#[test]
#[ignore = "explicit recovery timing workload"]
fn background_recovery_timing() -> Result<(), Box<dyn std::error::Error>> {
    use criterion::measurement::{Measurement, WallTime};
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("scene.recite");
    let source = recite_writer_model::workload::source(10_000, 0, 5);
    fs::write(&path, &source)?;
    let mut workbench = Workbench::new(&source)?;
    let baseline = workbench.document().source_snapshot();
    let mut store = RecoveryStore::open(&path)?;
    let mut queue_ms = Vec::new();
    for i in 0..100 {
        let start = WallTime.start();
        workbench.set_draft(format!("An uncommitted draft {i}"));
        store.queue(Some(Recovery::new(baseline.clone(), workbench.recovery())))?;
        queue_ms.push(WallTime.end(start).as_secs_f64() * 1000.);
    }
    let latest = Recovery::new(baseline, workbench.recovery());
    let start = WallTime.start();
    store.persist(Some(latest.clone()))?;
    let flush_ms = WallTime.end(start).as_secs_f64() * 1000.;
    drop(store);
    assert_eq!(RecoveryStore::open(&path)?.snapshot(), Some(&latest));
    println!(
        "{}",
        serde_json::json!({"passages":10_000, "source_bytes":source.len(), "drafts":100,
        "queue_ms":queue_ms, "flush_ms":flush_ms, "reopened_latest":true, "profile":"test"})
    );
    Ok(())
}

#[test]
fn recovery_owner_releases_lock_even_while_a_duplicate_descriptor_exists()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("scene.recite");
    let disk = RecoveryDisk::<Recovery>::open(&source)?;
    let duplicate = disk.lock.try_clone()?;
    assert!(matches!(
        RecoveryDisk::<Recovery>::open(&source),
        Err(FileError::RecoveryInUse)
    ));
    drop(disk);
    let reopened = RecoveryDisk::<Recovery>::open(&source)?;
    drop(reopened);
    drop(duplicate);
    Ok(())
}
