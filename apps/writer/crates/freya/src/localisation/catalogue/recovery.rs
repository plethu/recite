//! PO recovery retains the original source so changed disk content stays a conflict.
use super::*;
use crate::recovery::{Snapshot, SnapshotStore};
use serde::{Deserialize, Serialize};
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct SnapshotData {
    version: u32,
    source: String,
    drafts: Vec<(usize, Draft)>,
}
impl Snapshot for SnapshotData {
    fn valid_version(&self) -> bool {
        self.version == 1
    }
}
impl Catalogue {
    pub(crate) fn open_recoverable(path: &Path) -> Result<Self, String> {
        let mut value = Self::open(path)?;
        let store = SnapshotStore::<SnapshotData>::open(path).map_err(|e| e.to_string())?;
        if let Some(snapshot) = store.snapshot() {
            let document = PoDocument::parse_with_path(path.to_string_lossy(), &snapshot.source)
                .map_err(|e| e.to_string())?;
            value = Self::from_document(path, document);
            for (index, draft) in &snapshot.drafts {
                let id = PoEntryId::new(*index);
                let entry = value
                    .document
                    .entry(id)
                    .ok_or("Recovery refers to a missing PO entry")?;
                if entry.is_header() || entry.is_obsolete() {
                    return Err("Invalid recovered PO entry".into());
                }
                value.update(id, draft.clone());
            }
        }
        value.recovery = Some(store);
        Ok(value)
    }
    fn recovery_snapshot(&self) -> Option<SnapshotData> {
        self.dirty().then(|| SnapshotData {
            version: 1,
            source: self.document.source().to_owned(),
            drafts: self
                .drafts
                .iter()
                .map(|(id, d)| (id.index(), d.clone()))
                .collect(),
        })
    }
    pub(super) fn queue_recovery(&mut self) {
        let snapshot = self.recovery_snapshot();
        if let Some(store) = &mut self.recovery {
            self.recovery_failure = store.queue(snapshot).err().map(|e| e.to_string());
        }
    }
    pub(crate) fn flush_recovery(&mut self) -> Result<(), String> {
        let snapshot = self.recovery_snapshot();
        if let Some(store) = &mut self.recovery {
            store.persist(snapshot).map_err(|e| e.to_string())?;
        }
        self.recovery_failure = None;
        Ok(())
    }
    pub(crate) fn recovery_error(&self) -> Option<String> {
        self.recovery_failure
            .clone()
            .or_else(|| self.recovery.as_ref().and_then(SnapshotStore::error))
    }
    pub(super) fn adopt(&mut self, mut next: Self) -> Result<(), String> {
        next.recovery = self.recovery.take();
        *self = next;
        self.flush_recovery()
    }
}
