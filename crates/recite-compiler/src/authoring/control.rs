use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Host-owned interruption policy. The compiler knows neither protocol reasons
/// nor clocks, queues, or executors. Implementations must keep checkpoints cheap.
pub trait WorkControl {
    fn checkpoint(&self) -> Result<(), Interrupted>;
}

/// Cooperative stop bit shared by a host and one authoring operation.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    stopped: Arc<AtomicBool>,
}

/// No partial result from the interrupted operation may be published.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("authoring work interrupted")]
pub struct Interrupted;

impl CancellationToken {
    /// Creates an independent running operation.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Permanently requests interruption. Clones observe the same stop bit.
    pub fn interrupt(&self) {
        self.stopped.store(true, Ordering::Release);
    }

    /// Check at work boundaries and before publishing a result.
    pub fn checkpoint(&self) -> Result<(), Interrupted> {
        if self.stopped.load(Ordering::Acquire) {
            Err(Interrupted)
        } else {
            Ok(())
        }
    }
}
impl WorkControl for CancellationToken {
    fn checkpoint(&self) -> Result<(), Interrupted> {
        self.checkpoint()
    }
}

#[cfg(test)]
mod tests;
