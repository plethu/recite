//! Comparisons observe one exact local session and disk version without changing selection.
mod merging;
mod observation;
mod resolving;
use super::{FileError, ProjectFiles, read_regular};
use crate::recovery::Recovery;
pub use observation::{ExternalCheck, ExternalObservation};
use recite_writer_model::{NavigationStamp, ProjectContext, RecoveredDraft, Workbench};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalDecision {
    KeepWriter,
    UseInspectedDisk,
}
#[must_use = "Adoption can complete with incomplete recovery; inspect the receipt."]
pub struct ExternalResolution {
    pub archive: PathBuf,
    pub recovery_error: Option<FileError>,
}
pub struct ExternalComparison {
    path: PathBuf,
    recovery: Recovery,
    pending: bool,
    disk: Arc<str>,
    origin: NavigationStamp,
    context: Arc<ProjectContext>,
    archive: Option<PathBuf>,
    artifact: Option<super::SourceRepairArtifact>,
}
impl ExternalComparison {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn writer_source(&self) -> &str {
        self.recovery.draft.source()
    }
    pub fn disk_source(&self) -> &str {
        &self.disk
    }
    pub fn pending_draft(&self) -> Option<&RecoveredDraft> {
        self.pending.then_some(&self.recovery.draft)
    }
    pub fn archive(&self) -> Option<&Path> {
        self.archive.as_deref()
    }
    pub fn artifact(&self) -> Option<&super::SourceRepairArtifact> {
        self.artifact.as_ref()
    }
    /// Cheap local freshness check for presentation; adoption also rechecks the disk contents.
    pub fn is_current(&self, files: &ProjectFiles, model: &Workbench) -> bool {
        self.path == files.current
            && self.origin.is_current(model)
            && Arc::ptr_eq(&self.recovery.baseline, &files.saved)
            && Arc::ptr_eq(&self.context, &files.context)
    }
}
impl ProjectFiles {
    pub fn compare_external(&mut self, model: &Workbench) -> Result<(), FileError> {
        self.external = Some(self.external_comparison(model)?);
        Ok(())
    }
    /// Prepare a detached observation; callers can discard it without replacing a live inspection.
    pub fn external_comparison(&self, model: &Workbench) -> Result<ExternalComparison, FileError> {
        self.require_idle_save()?;
        self.require_navigation_ready()?;
        let disk = read_regular(&self.current)?.into();
        Ok(ExternalComparison {
            path: self.current.clone(),
            recovery: Recovery::new(self.saved.clone(), model.recovery()),
            pending: model.has_draft(),
            disk,
            origin: model.navigation_stamp(),
            context: self.context.clone(),
            archive: None,
            artifact: None,
        })
    }
}

#[cfg(test)]
mod tests;
