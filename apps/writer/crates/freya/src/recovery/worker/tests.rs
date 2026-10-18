use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Draft(usize);
impl Snapshot for Draft {
    type Encoding = usize;
    fn valid_version(&self) -> bool {
        true
    }
    fn encode(
        &self,
        output: impl std::io::Write,
        encoding: &mut usize,
    ) -> Result<(), serde_json::Error> {
        assert_ne!(self.0, usize::MAX, "test encoding failure");
        *encoding += 1;
        serde_json::to_writer(output, self)
    }
}

#[test]
fn forty_parked_stores_keep_durable_values_locks_and_caches_without_worker_threads()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let mut stores = Vec::new();
    for index in 0..40 {
        let path = dir.path().join(format!("{index}.recite"));
        let mut store = SnapshotStore::<Draft>::open(&path)?;
        assert!(
            store.worker.is_none(),
            "opening alone must not start a thread"
        );
        store.persist(Some(Draft(index)))?;
        assert!(store.worker.is_some());
        store.park()?;
        assert!(
            store.worker.is_none(),
            "park joins the worker before returning"
        );
        assert_eq!(
            store
                .disk
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .encoding,
            1
        );
        assert!(matches!(
            SnapshotStore::<Draft>::open(&path),
            Err(FileError::RecoveryInUse)
        ));
        stores.push(store);
    }
    assert_eq!(stores.iter().filter(|s| s.worker.is_some()).count(), 0);
    for (index, store) in stores.iter_mut().enumerate() {
        store.queue(Some(Draft(index + 100)))?;
        store.queue(Some(Draft(index + 200)))?;
        store.park()?;
        assert!(store.worker.is_none());
        assert!(!store.pending());
        let disk = store
            .disk
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(disk.snapshot(), Some(&Draft(index + 200)));
        assert!(disk.encoding >= 2, "encoding cache survives restart");
    }
    drop(stores);
    for index in 0..40 {
        let path = dir.path().join(format!("{index}.recite"));
        let mut store = SnapshotStore::<Draft>::open(&path)?;
        assert_eq!(store.snapshot(), Some(&Draft(index + 200)));
        store.persist(Some(Draft(index + 200)))?;
        assert!(
            store.worker.is_none(),
            "already-durable values need no worker"
        );
    }
    Ok(())
}

#[test]
fn parked_io_failure_keeps_ownership_and_retries_before_clear()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("scene.recite");
    let sidecar = super::super::sidecar(&path, ".recite-editor-recovery.json");
    let mut store = SnapshotStore::<Draft>::open(&path)?;
    std::fs::create_dir(&sidecar)?;
    store.queue(Some(Draft(1)))?;
    assert!(matches!(
        store.park(),
        Err(FileError::BackgroundRecovery(_))
    ));
    assert!(store.worker.is_none());
    assert!(store.pending());
    assert!(matches!(
        SnapshotStore::<Draft>::open(&path),
        Err(FileError::RecoveryInUse)
    ));
    std::fs::remove_dir(&sidecar)?;
    store.persist(Some(Draft(1)))?;
    store.park()?;
    assert!(!store.pending());
    assert_eq!(
        store
            .disk
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .snapshot(),
        Some(&Draft(1))
    );
    store.queue(Some(Draft(2)))?;
    store.persist(None)?;
    store.park()?;
    assert!(!sidecar.exists());
    drop(store);
    assert!(SnapshotStore::<Draft>::open(&path)?.snapshot().is_none());
    Ok(())
}

#[test]
fn dropping_an_active_store_drains_work_and_releases_the_lock_before_reopen()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("scene.recite");
    let mut store = SnapshotStore::<Draft>::open(&path)?;
    store.queue(Some(Draft(42)))?;
    drop(store);
    let reopened = SnapshotStore::<Draft>::open(&path)?;
    assert_eq!(reopened.snapshot(), Some(&Draft(42)));
    Ok(())
}

#[test]
fn parked_panic_is_distinct_from_idle_and_drop_releases_the_failed_owner()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("scene.recite");
    let mut store = SnapshotStore::<Draft>::open(&path)?;
    store.queue(Some(Draft(usize::MAX)))?;
    assert!(matches!(
        store.park(),
        Err(FileError::BackgroundRecovery(_))
    ));
    assert!(matches!(
        store
            .pending
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .phase,
        Phase::Failed
    ));
    assert!(store.worker.is_none());
    assert!(store.queue(Some(Draft(42))).is_err());
    assert!(matches!(
        SnapshotStore::<Draft>::open(&path),
        Err(FileError::RecoveryInUse)
    ));
    drop(store);
    let mut reopened = SnapshotStore::<Draft>::open(&path)?;
    assert!(reopened.snapshot().is_none());
    reopened.persist(Some(Draft(42)))?;
    Ok(())
}

#[test]
fn detached_durability_fence_waits_without_borrowing_the_live_store()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("scene.recite");
    let mut store = SnapshotStore::<Draft>::open(&path)?;
    // Hold real disk ownership so the recovery write cannot finish yet.
    let disk = store.disk.clone();
    let guard = disk.lock().unwrap();
    let ticket = store.begin_persist(Some(Draft(1)))?;
    let (done, result) = std::sync::mpsc::sync_channel(1);
    let waiter = std::thread::spawn(move || done.send(ticket.wait()).unwrap());
    assert!(matches!(
        result.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    ));
    // The UI owner remains free to queue newer content while durability waits.
    store.queue(Some(Draft(2)))?;
    drop(guard);
    result.recv_timeout(std::time::Duration::from_secs(5))??;
    waiter.join().unwrap();
    store.park()?;
    drop(store);
    drop(disk);
    assert_eq!(
        SnapshotStore::<Draft>::open(&path)?.snapshot(),
        Some(&Draft(2))
    );
    Ok(())
}

#[test]
fn async_parking_coalesces_without_reviving_worker_and_retains_latest_on_drop()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("scene.recite");
    let mut store = SnapshotStore::<Draft>::open(&path)?;
    let disk = store.disk.clone();
    let guard = disk.lock().unwrap();
    store.queue(Some(Draft(1)))?;
    let parking = store.begin_park();
    assert!(store.worker.is_none());
    store.queue(Some(Draft(2)))?;
    assert!(
        store.worker.is_none(),
        "parking must not start a second worker"
    );
    assert!(matches!(
        store.pending.0.lock().unwrap().phase,
        Phase::Parking
    ));
    let waiter = std::thread::spawn(move || parking.wait());
    drop(guard);
    waiter.join().map_err(|_| "parking panicked")??;
    assert!(!store.pending());
    assert!(matches!(
        store.pending.0.lock().unwrap().phase,
        Phase::Parked
    ));
    assert_eq!(disk.lock().unwrap().snapshot(), Some(&Draft(2)));
    // The old worker already exited. A newer snapshot must own a real successor,
    // and dropping the store without another queue must still drain that value.
    store.queue(Some(Draft(3)))?;
    assert!(store.worker.is_some());
    drop(store);
    drop(disk);
    assert_eq!(
        SnapshotStore::<Draft>::open(&path)?.snapshot(),
        Some(&Draft(3))
    );
    Ok(())
}

#[test]
fn dropping_parking_receipt_does_not_discard_a_queued_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("scene.recite");
    let mut store = SnapshotStore::<Draft>::open(&path)?;
    let disk = store.disk.clone();
    let guard = disk.lock().unwrap();
    store.queue(Some(Draft(7)))?;
    let parking = store.begin_park();
    store.queue(Some(Draft(8)))?;
    drop(parking);
    drop(guard);
    // A detached worker keeps the disk lease and drains even if the UI tears down.
    // Holding our disk Arc lets the test wait for its acknowledged phase explicitly.
    let pending = store.pending.clone();
    drop(store);
    let mut state = pending.0.lock().unwrap();
    while state.completed < 2 && state.phase != Phase::Failed {
        let (next, timeout) = pending
            .1
            .wait_timeout(state, std::time::Duration::from_secs(5))
            .unwrap();
        state = next;
        assert!(!timeout.timed_out(), "recovery did not drain");
    }
    assert_eq!(disk.lock().unwrap().snapshot(), Some(&Draft(8)));
    Ok(())
}
