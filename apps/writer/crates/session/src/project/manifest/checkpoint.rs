//! Rename's commit point is an observed atomic replacement, before directory durability.
use super::*;
use std::io::Write;

pub(in crate::project) enum Checkpoint {
    Durable,
    WrittenWithWarning(FileError),
}
impl Checkpoint {
    pub(in crate::project) fn result(self) -> Result<(), FileError> {
        match self {
            Self::Durable => Ok(()),
            Self::WrittenWithWarning(error) => Err(error),
        }
    }
}
impl ManifestDraft {
    pub(in crate::project) fn checkpoint(
        &mut self,
        text: String,
        documents: std::collections::BTreeMap<String, crate::recovery::Recovery>,
        affected: Vec<String>,
    ) -> Result<Checkpoint, FileError> {
        self.checkpoint_with_sync(text, documents, affected, sync_directory)
    }
    pub(in crate::project) fn checkpoint_with_sync(
        &mut self,
        text: String,
        documents: std::collections::BTreeMap<String, crate::recovery::Recovery>,
        affected: Vec<String>,
        sync: impl FnOnce(&Path) -> io::Result<()>,
    ) -> Result<Checkpoint, FileError> {
        let draft = Draft {
            version: 1,
            documents,
            affected,
            baseline: self.draft.baseline.clone(),
            text,
        };
        let parent = self.recovery.parent().ok_or(FileError::Selection)?;
        if draft.text == draft.baseline && draft.affected.is_empty() {
            match std::fs::remove_file(&self.recovery) {
                Ok(()) => (),
                Err(error) if error.kind() == io::ErrorKind::NotFound => (),
                Err(error) => return Err(error.into()),
            }
        } else {
            let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
            serde_json::to_writer(&mut temporary, &draft)?;
            temporary.flush()?;
            temporary.as_file().sync_all()?;
            temporary
                .persist(&self.recovery)
                .map_err(|error| error.error)?;
        }
        // Every fallible preparation is complete. A later fsync failure cannot undo replacement.
        self.draft = draft;
        self.state = PersistenceState::CheckpointCleanupPending;
        match sync(parent) {
            Ok(()) => {
                self.state = PersistenceState::Editable;
                Ok(Checkpoint::Durable)
            }
            Err(error) => Ok(Checkpoint::WrittenWithWarning(error.into())),
        }
    }
}

#[cfg(test)]
mod tests;
