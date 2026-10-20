use super::*;

impl ProjectFiles {
    pub fn save_retained(&mut self) -> Result<(), FileError> {
        self.require_idle_save()?;
        let paths: Vec<PathBuf> = self.retained.keys().cloned().collect();
        for path in paths {
            let session = self.retained.get_mut(&path).ok_or(FileError::Selection)?;
            let outcome = session.save(&path);
            let source = session.baseline.clone();
            let record = self.record_saved(&path, &source);
            outcome.and(record)?;
        }
        if self.manifest.dirty() {
            self.manifest.save()?;
        }
        Ok(())
    }
}

impl Retained {
    pub(super) fn save(&mut self, path: &Path) -> Result<(), FileError> {
        self.model.apply()?;
        self.save_applied(path)
    }
    pub(super) fn save_applied(&mut self, path: &Path) -> Result<(), FileError> {
        crate::project::save::replace_checked(
            path,
            &self.baseline,
            self.model.document().source(),
        )?;
        self.baseline = self.model.document().source_snapshot();
        let recovery = self
            .model
            .has_draft()
            .then(|| crate::recovery::Recovery::new(self.baseline.clone(), self.model.recovery()));
        self.persist_recovery(recovery)
    }
}
