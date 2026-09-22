//! The manifest draft participates in reviewed project edits and recovery.
use super::{FileError, read_regular, save::replace_checked};
use std::{
    fs::{File, OpenOptions},
    io::Write,
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
        let draft: Draft = if recovery.exists() {
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
        })
    }
    pub fn text(&self) -> &str {
        &self.draft.text
    }
    pub fn dirty(&self) -> bool {
        self.draft.text != self.draft.baseline || !self.draft.affected.is_empty()
    }
    pub fn affected(&self) -> &[String] {
        &self.draft.affected
    }
    pub fn pending(&self) -> &std::collections::BTreeMap<String, crate::recovery::Recovery> {
        &self.draft.documents
    }
    pub fn checkpointed(&mut self) -> Result<(), FileError> {
        self.set(
            self.draft.text.clone(),
            Default::default(),
            self.draft.affected.clone(),
        )
    }
    pub fn set(
        &mut self,
        text: String,
        documents: std::collections::BTreeMap<String, crate::recovery::Recovery>,
        affected: Vec<String>,
    ) -> Result<(), FileError> {
        let draft = Draft {
            version: 1,
            documents,
            affected,
            baseline: self.draft.baseline.clone(),
            text,
        };
        if draft.text == draft.baseline && draft.affected.is_empty() {
            if self.recovery.exists() {
                std::fs::remove_file(&self.recovery)?;
            }
            self.draft = draft;
            return Ok(());
        }
        let mut file = atomic_write_file::AtomicWriteFile::open(&self.recovery)?;
        file.write_all(&serde_json::to_vec(&draft)?)?;
        file.commit().map_err(FileError::Commit)?;
        #[cfg(unix)]
        File::open(self.path.parent().ok_or(FileError::Selection)?)?.sync_all()?;
        self.draft = draft;
        Ok(())
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
    pub fn save(&mut self) -> Result<(), FileError> {
        replace_checked(&self.path, &self.draft.baseline, &self.draft.text)?;
        self.draft.baseline = self.draft.text.clone();
        self.draft.affected.clear();
        self.draft.documents.clear();
        if self.recovery.exists() {
            std::fs::remove_file(&self.recovery)?;
        }
        Ok(())
    }
}
