//! Parking transfers the durability fence and thread join without releasing the disk lease.
use super::{Durability, Phase, SnapshotStore, fail};
use crate::{
    project::FileError,
    recovery::{RecoveryDisk, Snapshot},
};
use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

pub(crate) struct Parking<T: Snapshot> {
    durability: Durability<T>,
    worker: Option<JoinHandle<()>>,
    _disk: Arc<Mutex<RecoveryDisk<T>>>,
}
impl<T: Snapshot> SnapshotStore<T> {
    pub(crate) fn begin_park(&mut self) -> Parking<T> {
        let mut state = self.pending.0.lock().unwrap_or_else(|e| e.into_inner());
        if self.worker.is_some() && state.phase == Phase::Running {
            state.phase = Phase::Parking;
        }
        self.pending.1.notify_all();
        Parking {
            durability: self.durability(),
            worker: self.worker.take(),
            _disk: self.disk.clone(),
        }
    }
}
impl<T: Snapshot> Parking<T> {
    pub(crate) fn wait(self) -> Result<(), FileError> {
        let pending = self.durability.pending.clone();
        let result = self.durability.wait();
        if let Some(worker) = self.worker
            && worker.join().is_err()
        {
            fail(&pending);
        }
        let error = pending
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .error
            .clone();
        result.and(error.map_or(Ok(()), |error| Err(FileError::BackgroundRecovery(error))))
    }
}
