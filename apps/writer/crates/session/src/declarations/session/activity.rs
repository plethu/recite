//! One canonical activity gates schema, source, save and close lifecycles.
use crate::declarations::producer::Job;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(super) enum Activity {
    Idle,
    StartingGeneration(ActivityToken),
    Generating(Job),
    PublishingGeneration(ActivityToken),
    ChangingSource(ActivityToken),
    RecoveringSource(ActivityToken),
}
impl Activity {
    pub fn cancel(&self) {
        match self {
            Self::Idle => (),
            Self::Generating(job) => job.cancel(),
            Self::StartingGeneration(publication)
            | Self::PublishingGeneration(publication)
            | Self::ChangingSource(publication)
            | Self::RecoveringSource(publication) => publication.cancel(),
        }
    }
}
impl Drop for Activity {
    fn drop(&mut self) {
        self.cancel();
    }
}
/// Identity belongs to this activity, independently of the visible screen or draft.
#[derive(Clone)]
pub(super) struct ActivityToken(Arc<AtomicBool>);
impl ActivityToken {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
    pub fn matches(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
