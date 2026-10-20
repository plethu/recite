//! One disk owner, one pending snapshot. Explicit persistence waits for durability.
mod parking;
use super::{RecoveryDisk, Snapshot};
use crate::FileError;
pub use parking::Parking;
use std::{
    path::Path,
    sync::{Arc, Condvar, Mutex},
    thread::JoinHandle,
};

/// Process-local durability ordering; unrelated to document or declaration revisions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct RecoverySequence(u64);
impl RecoverySequence {
    fn checked_next(self) -> Result<Self, FileError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or_else(|| FileError::BackgroundRecovery("Recovery sequence exhausted".into()))
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Phase {
    Running,
    Parked,
    Parking,
    Failed,
    Closed,
}
struct Pending<T: Snapshot> {
    next: Option<(RecoverySequence, Option<T>)>,
    completed: RecoverySequence,
    error: Option<String>,
    phase: Phase,
}
pub struct SnapshotStore<T: Snapshot> {
    snapshot: Option<T>,
    sequence: RecoverySequence,
    pending: Arc<(Mutex<Pending<T>>, Condvar)>,
    disk: Arc<Mutex<RecoveryDisk<T>>>,
    worker: Option<JoinHandle<()>>,
}
/// A fixed durability fence. Later coalesced snapshots also satisfy this sequence.
pub struct Durability<T: Snapshot> {
    pending: Arc<(Mutex<Pending<T>>, Condvar)>,
    sequence: RecoverySequence,
}
impl<T: Snapshot> Durability<T> {
    pub fn wait(self) -> Result<(), FileError> {
        let mut state = self.pending.0.lock().unwrap_or_else(|e| e.into_inner());
        while state.completed < self.sequence && state.phase != Phase::Failed {
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
impl<T: Snapshot> SnapshotStore<T> {
    pub fn parked_at(&self, sequence: RecoverySequence) -> bool {
        let state = self.pending.0.lock().unwrap_or_else(|e| e.into_inner());
        self.sequence == sequence
            && self.worker.is_none()
            && state.phase == Phase::Parked
            && state.next.is_none()
            && state.completed >= sequence
            && state.error.is_none()
    }
    pub fn sequence(&self) -> RecoverySequence {
        self.sequence
    }
    pub fn durability(&self) -> Durability<T> {
        Durability {
            pending: self.pending.clone(),
            sequence: self.sequence,
        }
    }
    pub fn begin_persist(&mut self, snapshot: Option<T>) -> Result<Durability<T>, FileError> {
        self.submit(snapshot)?;
        Ok(self.durability())
    }
    pub fn open(source: &Path) -> Result<Self, FileError> {
        let disk = RecoveryDisk::open(source)?;
        let snapshot = disk.snapshot().cloned();
        Ok(Self {
            snapshot,
            sequence: RecoverySequence::default(),
            pending: Arc::new((
                Mutex::new(Pending {
                    next: None,
                    completed: RecoverySequence::default(),
                    error: None,
                    phase: Phase::Parked,
                }),
                Condvar::new(),
            )),
            disk: Arc::new(Mutex::new(disk)),
            worker: None,
        })
    }
    fn start(&mut self, state: &mut Pending<T>) -> Result<(), FileError> {
        if self.worker.is_none() {
            let shared = Arc::clone(&self.pending);
            let disk = Arc::clone(&self.disk);
            self.worker = Some(
                std::thread::Builder::new()
                    .name("recite-recovery".into())
                    .spawn(move || {
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            run(&disk, &shared)
                        }));
                        if result.is_err() {
                            fail(&shared);
                        }
                    })?,
            );
            state.phase = Phase::Running;
        }
        Ok(())
    }
    /// Drain work before releasing the thread; retain the disk lock and encoding cache.
    pub fn park(&mut self) -> Result<(), FileError> {
        self.begin_park().wait()
    }
    fn stop(&mut self, phase: Phase) {
        let mut state = self.pending.0.lock().unwrap_or_else(|e| e.into_inner());
        if state.phase != Phase::Failed {
            state.phase = phase;
        }
        self.pending.1.notify_all();
        drop(state);
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            fail(&self.pending);
        }
    }
    pub fn error(&self) -> Option<String> {
        self.pending
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .error
            .clone()
    }
    pub fn pending(&self) -> bool {
        let state = self.pending.0.lock().unwrap_or_else(|e| e.into_inner());
        state.completed < self.sequence || state.error.is_some()
    }
    pub fn snapshot(&self) -> Option<&T> {
        self.snapshot.as_ref()
    }
    pub fn queue(&mut self, snapshot: Option<T>) -> Result<(), FileError> {
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
    fn submit(&mut self, snapshot: Option<T>) -> Result<(), FileError> {
        let pending = Arc::clone(&self.pending);
        let mut state = pending.0.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(state.phase, Phase::Failed | Phase::Closed) {
            return Err(FileError::BackgroundRecovery(
                state
                    .error
                    .clone()
                    .unwrap_or_else(|| "Recovery is closed".into()),
            ));
        }
        if snapshot != self.snapshot || state.error.is_some() {
            let sequence = self.sequence.checked_next()?;
            if state.phase != Phase::Parking {
                self.start(&mut state)?;
            }
            self.sequence = sequence;
            self.snapshot = snapshot.clone();
            state.next = Some((self.sequence, snapshot));
            self.pending.1.notify_one();
        }
        Ok(())
    }
    pub fn persist(&mut self, snapshot: Option<T>) -> Result<(), FileError> {
        self.submit(snapshot)?;
        self.flush()
    }
    fn flush(&self) -> Result<(), FileError> {
        self.durability().wait()
    }
}
impl<T: Snapshot> Drop for SnapshotStore<T> {
    fn drop(&mut self) {
        // Save/close call persist explicitly to report errors. Drop still drains queued work.
        self.stop(Phase::Closed);
        // A transferred Parking receipt keeps the disk lease until its worker exits.
        // Otherwise stop joins here before the disk owner releases the OS lock.
    }
}
fn fail<T: Snapshot>(shared: &(Mutex<Pending<T>>, Condvar)) {
    let mut state = shared.0.lock().unwrap_or_else(|e| e.into_inner());
    state.phase = Phase::Failed;
    state.error =
        Some("Recovery worker stopped unexpectedly; keep the editor open and save a copy.".into());
    shared.1.notify_all();
}

fn run<T: Snapshot>(disk: &Mutex<RecoveryDisk<T>>, shared: &(Mutex<Pending<T>>, Condvar)) {
    loop {
        let mut state = shared.0.lock().unwrap_or_else(|e| e.into_inner());
        while state.next.is_none() && state.phase == Phase::Running {
            state = shared.1.wait(state).unwrap_or_else(|e| e.into_inner());
        }
        let Some((sequence, snapshot)) = state.next.take() else {
            // Publish exit while holding the queue lock. A later submit can start
            // a successor, but cannot revive this already-returning worker.
            if state.phase == Phase::Parking {
                state.phase = Phase::Parked;
            }
            shared.1.notify_all();
            return;
        };
        drop(state);
        let result = disk
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .persist(snapshot);
        let mut state = shared.0.lock().unwrap_or_else(|e| e.into_inner());
        state.completed = sequence;
        state.error = result.err().map(|error| error.to_string());
        shared.1.notify_all();
    }
}

#[cfg(test)]
mod tests;
