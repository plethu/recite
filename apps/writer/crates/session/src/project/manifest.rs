//! The manifest draft participates in reviewed project edits and recovery.
use super::{
    FileError, read_regular,
    save::{replace_checked, sync_directory},
};
mod checkpoint;
pub(super) use checkpoint::Checkpoint;
use std::{
    fs::{File, OpenOptions},
    io,
    path::{Path, PathBuf},
};
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Draft {
    version: u32,
    #[serde(default)]
    documents: std::collections::BTreeMap<String, crate::recovery::Recovery>,
    #[serde(default)]
    affected: Vec<String>,
    baseline: String,
    text: String,
}
pub(super) struct ManifestDraft {
    path: PathBuf,
    recovery: PathBuf,
    _lease: File,
    draft: Draft,
    state: PersistenceState,
}

impl Drop for ManifestDraft {
    fn drop(&mut self) {
        // A concurrent process spawn can briefly inherit the descriptor before exec.
        // Ownership ends here even while that duplicate descriptor is still alive.
        let _ = self._lease.unlock();
    }
}

#[derive(PartialEq, Eq)]
enum PersistenceState {
    Editable,
    CheckpointCleanupPending,
    SaveCleanupPending,
}
impl ManifestDraft {
    pub fn open(root: &Path) -> Result<Self, FileError> {
        let path = root.join("recite.project.toml");
        let recovery = root.join(".recite-manifest-draft.json");
        let lease = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join(".recite-manifest-draft.lock"))?;
        lease.try_lock().map_err(|_| FileError::RecoveryInUse)?;
        let recovered = recovery.exists();
        let draft: Draft = if recovered {
            serde_json::from_str(&read_regular(&recovery)?)?
        } else {
            let text = read_regular(&path)?;
            Draft {
                version: 1,
                documents: Default::default(),
                affected: Vec::new(),
                baseline: text.clone(),
                text,
            }
        };
        if draft.version != 1 {
            return Err(FileError::RecoveryVersion);
        }
        Ok(Self {
            path,
            recovery,
            _lease: lease,
            draft,
            state: if recovered {
                PersistenceState::CheckpointCleanupPending
            } else {
                PersistenceState::Editable
            },
        })
    }
    pub fn text(&self) -> &str {
        &self.draft.text
    }
    pub fn dirty(&self) -> bool {
        self.draft.text != self.draft.baseline
            || !self.draft.affected.is_empty()
            || self.state != PersistenceState::Editable
    }
    pub fn affected(&self) -> &[String] {
        &self.draft.affected
    }
    pub fn pending(&self) -> &std::collections::BTreeMap<String, crate::recovery::Recovery> {
        &self.draft.documents
    }
    pub fn recovery_pending(&self) -> bool {
        !self.draft.documents.is_empty() || self.state == PersistenceState::CheckpointCleanupPending
    }
    pub fn checkpointed(&mut self) -> Result<(), FileError> {
        self.checkpointed_with_sync(sync_directory)
    }
    pub(super) fn checkpointed_with_sync(
        &mut self,
        sync: impl FnOnce(&Path) -> io::Result<()>,
    ) -> Result<(), FileError> {
        self.checkpoint_with_sync(
            self.draft.text.clone(),
            Default::default(),
            self.draft.affected.clone(),
            sync,
        )?
        .result()
    }
    pub fn refresh(&mut self, text: String) {
        if !self.dirty() {
            self.draft = Draft {
                version: 1,
                documents: Default::default(),
                affected: Vec::new(),
                baseline: text.clone(),
                text,
            };
        }
    }

    pub fn save_request(&self) -> ManifestSave {
        ManifestSave {
            path: self.path.clone(),
            recovery: self.recovery.clone(),
            baseline: self.draft.baseline.clone(),
            text: self.draft.text.clone(),
        }
    }
    pub fn accept_save(&mut self, outcome: ManifestOutcome) -> Result<(), FileError> {
        if outcome.replaced {
            self.draft.baseline = self.draft.text.clone();
            self.draft.affected.clear();
            self.draft.documents.clear();
            self.state = PersistenceState::SaveCleanupPending;
            if outcome.result.is_ok() {
                self.state = PersistenceState::Editable;
            }
        }
        outcome.result
    }
}

#[cfg(test)]
mod tests;

pub struct ManifestSave {
    path: PathBuf,
    recovery: PathBuf,
    baseline: String,
    text: String,
}
pub struct ManifestOutcome {
    replaced: bool,
    result: Result<(), FileError>,
}
impl ManifestSave {
    pub fn execute(self) -> ManifestOutcome {
        self.execute_with_sync(sync_directory)
    }
    fn execute_with_sync(self, sync: impl FnOnce(&Path) -> io::Result<()>) -> ManifestOutcome {
        let mut replaced = false;
        let result = (|| {
            replace_checked(&self.path, &self.baseline, &self.text)?;
            replaced = true;
            match std::fs::remove_file(&self.recovery) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            sync(self.path.parent().ok_or(FileError::Selection)?)?;
            Ok(())
        })();
        ManifestOutcome { replaced, result }
    }
}
