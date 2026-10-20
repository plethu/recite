//! Standalone declaration drafts use the shared background recovery store.
use crate::recovery::worker;
use crate::{
    project::FileError,
    recovery::{Snapshot, SnapshotStore},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftSnapshot {
    version: u32,
    pub baseline: String,
    pub draft: String,
}
impl Snapshot for DraftSnapshot {
    type Encoding = ();

    fn valid_version(&self) -> bool {
        self.version == 1
    }
}
impl super::session::Source {
    pub(super) fn begin_replacement_recovery(
        &mut self,
        baseline: &str,
        draft: &str,
    ) -> Result<worker::Durability<DraftSnapshot>, FileError> {
        self.recovery
            .begin_persist((baseline != draft).then(|| DraftSnapshot {
                version: 1,
                baseline: baseline.to_owned(),
                draft: draft.to_owned(),
            }))
    }
    /// Draft changes retain the mounted buffer identity; rebasing advances it separately.
    pub fn set_draft(&mut self, draft: String) -> Result<(), FileError> {
        if self.draft != draft {
            self.draft = draft;
            self.queue_recovery()?;
        }
        Ok(())
    }
    fn snapshot(&self) -> Option<DraftSnapshot> {
        (self.draft != self.baseline).then(|| DraftSnapshot {
            version: 1,
            baseline: self.baseline.clone(),
            draft: self.draft.clone(),
        })
    }
    pub fn begin_saved_recovery(&mut self) -> Result<worker::Durability<DraftSnapshot>, FileError> {
        self.recovery.begin_persist(self.snapshot())
    }
    pub fn queue_recovery(&mut self) -> Result<(), FileError> {
        self.recovery.queue(self.snapshot())
    }
    pub fn flush_recovery(&mut self) -> Result<(), FileError> {
        self.recovery.persist(self.snapshot())
    }
    pub fn recovery_error(&self) -> Option<String> {
        self.recovery.error()
    }
}
pub type Store = SnapshotStore<DraftSnapshot>;
