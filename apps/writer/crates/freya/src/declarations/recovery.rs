//! Standalone declaration drafts use the shared background recovery store.
use crate::{
    project::FileError,
    recovery::{Snapshot, SnapshotStore},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct DraftSnapshot {
    version: u32,
    pub baseline: String,
    pub draft: String,
}
impl Snapshot for DraftSnapshot {
    fn valid_version(&self) -> bool {
        self.version == 1
    }
}
impl super::session::Source {
    fn snapshot(&self) -> Option<DraftSnapshot> {
        (self.draft != self.baseline).then(|| DraftSnapshot {
            version: 1,
            baseline: self.baseline.clone(),
            draft: self.draft.clone(),
        })
    }
    pub(super) fn queue_recovery(&mut self) -> Result<(), FileError> {
        self.recovery.queue(self.snapshot())
    }
    pub(crate) fn flush_recovery(&mut self) -> Result<(), FileError> {
        self.recovery.persist(self.snapshot())
    }
    pub(crate) fn recovery_error(&self) -> Option<String> {
        self.recovery.error()
    }
}
pub(super) type Store = SnapshotStore<DraftSnapshot>;
