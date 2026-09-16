//! One disk owner, one pending snapshot. Explicit persistence waits for durability.
use super::{Recovery, RecoveryDisk};
use crate::project::FileError;
use std::{
    path::Path,
    sync::{Arc, Condvar, Mutex},
    thread::JoinHandle,
};

#[derive(Default)]
struct Pending {
    next: Option<(u64, Option<Recovery>)>,
    completed: u64,
    error: Option<String>,
    stopping: bool,
}
pub(crate) struct RecoveryStore {
    snapshot: Option<Recovery>,
    sequence: u64,
    pending: Arc<(Mutex<Pending>, Condvar)>,
    worker: Option<JoinHandle<()>>,
}
impl RecoveryStore {
    pub fn open(source: &Path) -> Result<Self, FileError> {
        let mut disk = RecoveryDisk::open(source)?;
        let snapshot = disk.snapshot().cloned();
        let pending = Arc::new((Mutex::new(Pending::default()), Condvar::new()));
        let shared = pending.clone();
        let worker = std::thread::Builder::new().name("recite-recovery".into()).spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&mut disk, &shared)));
            if result.is_err() {
                let mut state = shared.0.lock().unwrap_or_else(|e| e.into_inner());
                state.error = Some("Recovery worker stopped unexpectedly; keep the editor open and save a copy.".into());
                state.stopping = true;
                shared.1.notify_all();
            }
        })?;
        Ok(Self {
            snapshot,
            sequence: 0,
            pending,
            worker: Some(worker),
        })
    }
    pub fn error(&self) -> Option<String> {
        self.pending
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .error
            .clone()
    }
    pub fn snapshot(&self) -> Option<&Recovery> {
        self.snapshot.as_ref()
    }
    pub fn queue(&mut self, snapshot: Option<Recovery>) -> Result<(), FileError> {
        let previous_error = self
            .pending
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .error
            .clone();
        self.submit(snapshot)?;
        previous_error.map_or(Ok(()), |error| Err(FileError::BackgroundRecovery(error)))
    }
    fn submit(&mut self, snapshot: Option<Recovery>) -> Result<(), FileError> {
        let mut state = self.pending.0.lock().unwrap_or_else(|e| e.into_inner());
        if state.stopping {
            return Err(FileError::BackgroundRecovery(
                state
                    .error
                    .clone()
                    .unwrap_or_else(|| "Recovery is closed".into()),
            ));
        }
        if snapshot != self.snapshot || state.error.is_some() {
            self.sequence = self.sequence.checked_add(1).ok_or_else(|| {
                FileError::BackgroundRecovery("Recovery sequence exhausted".into())
            })?;
            self.snapshot = snapshot.clone();
            state.next = Some((self.sequence, snapshot));
            self.pending.1.notify_one();
        }
        Ok(())
    }
    pub fn persist(&mut self, snapshot: Option<Recovery>) -> Result<(), FileError> {
        self.submit(snapshot)?;
        self.flush()
    }
    fn flush(&self) -> Result<(), FileError> {
        let mut state = self.pending.0.lock().unwrap_or_else(|e| e.into_inner());
        while state.completed < self.sequence && !state.stopping {
            state = self
                .pending
                .1
                .wait(state)
                .unwrap_or_else(|e| e.into_inner());
        }
        state
            .error
            .clone()
            .map_or(Ok(()), |error| Err(FileError::BackgroundRecovery(error)))
    }
}
impl Drop for RecoveryStore {
    fn drop(&mut self) {
        // Save/close call persist explicitly to report errors. Drop still drains queued work.
        let mut state = self.pending.0.lock().unwrap_or_else(|e| e.into_inner());
        state.stopping = true;
        self.pending.1.notify_all();
        drop(state);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn run(disk: &mut RecoveryDisk, shared: &(Mutex<Pending>, Condvar)) {
    loop {
        let mut state = shared.0.lock().unwrap_or_else(|e| e.into_inner());
        while state.next.is_none() && !state.stopping {
            state = shared.1.wait(state).unwrap_or_else(|e| e.into_inner());
        }
        let Some((sequence, snapshot)) = state.next.take() else {
            return;
        };
        drop(state);
        let result = disk.persist(snapshot);
        let mut state = shared.0.lock().unwrap_or_else(|e| e.into_inner());
        state.completed = sequence;
        state.error = result.err().map(|error| error.to_string());
        shared.1.notify_all();
    }
}
