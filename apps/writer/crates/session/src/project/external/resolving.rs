//! Build replacement analysis first; publication never overwrites the inspected source file.
use super::*;
use crate::recovery::worker::Durability;
use recite_compiler::authoring::{CancellationToken, SavedDocument, WorkControl};
use recite_writer_model::{PreparedSourceReplacement, SearchIndex, SourceValidation};

struct Baseline {
    source: Arc<str>,
    context: Arc<ProjectContext>,
    search: Arc<SearchIndex>,
}
impl ProjectFiles {
    pub fn resolve_external(
        &mut self,
        model: &mut Workbench,
        decision: ExternalDecision,
    ) -> Result<ExternalResolution, FileError> {
        self.resolve_external_with_control(model, decision, &CancellationToken::new())
    }
    pub fn resolve_external_with_control(
        &mut self,
        model: &mut Workbench,
        decision: ExternalDecision,
        control: &dyn WorkControl,
    ) -> Result<ExternalResolution, FileError> {
        self.require_idle_save()?;
        self.require_navigation_ready()?;
        control.checkpoint()?;
        let comparison = self.checked_external(model)?;
        let replacement = match decision {
            ExternalDecision::KeepWriter => None,
            ExternalDecision::UseInspectedDisk => Some(
                model
                    .source_replacement_request(SourceValidation::Freeform)
                    .prepare(comparison.disk.clone().into(), control)?,
            ),
        };
        self.publish_external(model, replacement, control)
    }
    pub(super) fn publish_external(
        &mut self,
        model: &mut Workbench,
        replacement: Option<PreparedSourceReplacement>,
        control: &dyn WorkControl,
    ) -> Result<ExternalResolution, FileError> {
        self.publish_external_with_wait(model, replacement, control, Durability::wait)
    }
    pub(super) fn publish_external_with_wait(
        &mut self,
        model: &mut Workbench,
        mut replacement: Option<PreparedSourceReplacement>,
        control: &dyn WorkControl,
        wait: impl FnOnce(Durability<Recovery>) -> Result<(), FileError>,
    ) -> Result<ExternalResolution, FileError> {
        self.require_idle_save()?;
        self.require_navigation_ready()?;
        let comparison = self.checked_external(model)?;
        let baseline = self.external_baseline(model, comparison.disk.clone())?;
        control.checkpoint()?;
        let archive = crate::export_recovery(&self.current, &comparison.recovery)?;
        self.external.as_mut().ok_or(FileError::Selection)?.archive = Some(archive.clone());
        // A complete archive remains discoverable even if cancellation or a later check vetoes adoption.
        control.checkpoint()?;
        self.checked_external(model)?;
        if replacement
            .as_ref()
            .is_some_and(|replacement| !replacement.is_current(model))
        {
            return Err(FileError::Conflict);
        }
        let recovery = match &replacement {
            None => (model.has_draft() || model.document().source() != baseline.source.as_ref())
                .then(|| Recovery::new(baseline.source.clone(), model.recovery())),
            Some(replacement) => (replacement.recovery().source() != baseline.source.as_ref())
                .then(|| Recovery::new(baseline.source.clone(), replacement.recovery().clone())),
        };
        // Submission is the admission boundary. A later durability failure cannot un-adopt source.
        let durability = self.recovery.begin_persist(recovery)?;
        if let Some(replacement) = &mut replacement {
            model.accept_source_replacement(replacement)?;
        }
        self.saved = baseline.source;
        self.context = baseline.context;
        self.search = baseline.search;
        self.external = None;
        Ok(ExternalResolution {
            archive,
            recovery_error: wait(durability).err(),
        })
    }
    pub(super) fn checked_external(
        &self,
        model: &Workbench,
    ) -> Result<&ExternalComparison, FileError> {
        let comparison = self.external.as_ref().ok_or(FileError::Selection)?;
        if !comparison.is_current(self, model)
            || read_regular(&comparison.path)? != comparison.disk.as_ref()
        {
            return Err(FileError::Conflict);
        }
        Ok(comparison)
    }
    fn external_baseline(
        &self,
        model: &Workbench,
        source: Arc<str>,
    ) -> Result<Baseline, FileError> {
        let mut context = self.context.as_ref().clone();
        let saved = context
            .documents
            .iter_mut()
            .find(|document| document.key() == model.document().key())
            .ok_or(FileError::Selection)?;
        *saved = SavedDocument::from_shared(model.document().key().clone(), source.clone());
        let mut search = self.search.as_ref().clone();
        search.replace_document(saved.clone());
        Ok(Baseline {
            source,
            context: Arc::new(context),
            search: Arc::new(search),
        })
    }
}
