use super::{CancellationToken, Interrupted, WorkControl};
use std::sync::atomic::{AtomicUsize, Ordering};

pub(in crate::authoring) struct CheckpointBudget(AtomicUsize);
impl WorkControl for CheckpointBudget {
    fn checkpoint(&self) -> Result<(), Interrupted> {
        self.0
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
                remaining.checked_sub(1)
            })
            .map(|_| ())
            .map_err(|_| Interrupted)
    }
}
impl CancellationToken {
    pub(in crate::authoring) fn interrupt_after(checkpoints: usize) -> CheckpointBudget {
        CheckpointBudget(AtomicUsize::new(checkpoints))
    }
}
