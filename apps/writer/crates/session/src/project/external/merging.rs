//! The IDE edits an owned text artifact; exact local/disk comparison authority admits import.
use super::*;
use crate::project::{SourceRepairArtifact, SourceRepairExport};
use recite_compiler::authoring::WorkControl;
use recite_writer_model::{DraftSnapshot, EditOperation, SourceValidation, View};

impl ProjectFiles {
    pub fn export_external_merge(
        &mut self,
        model: &Workbench,
        control: &dyn WorkControl,
    ) -> Result<SourceRepairArtifact, FileError> {
        self.require_idle_save()?;
        self.require_navigation_ready()?;
        control.checkpoint()?;
        let comparison = self.checked_external(model)?;
        let source = merge_source(model, control)?;
        let export =
            SourceRepairExport::captured(self.current.clone(), comparison.recovery.clone(), source);
        control.checkpoint()?;
        let artifact = export.write()?;
        let comparison = self.external.as_mut().ok_or(FileError::Selection)?;
        comparison.archive = Some(artifact.recovery.clone());
        comparison.artifact = Some(artifact.clone());
        Ok(artifact)
    }
    pub fn import_external_merge(
        &mut self,
        model: &mut Workbench,
        path: &Path,
        control: &dyn WorkControl,
    ) -> Result<ExternalResolution, FileError> {
        self.require_idle_save()?;
        self.require_navigation_ready()?;
        control.checkpoint()?;
        self.checked_external(model)?;
        let source = read_regular(path)?;
        let replacement = model
            .source_replacement_request(SourceValidation::NoNewErrors)
            .prepare(source.into(), control)?;
        self.publish_external(model, Some(replacement), control)
    }
}
fn merge_source(model: &Workbench, control: &dyn WorkControl) -> Result<DraftSnapshot, FileError> {
    if model.view() == &View::Source {
        return Ok(model.draft_snapshot().clone());
    }
    let mut snapshot = model.navigation_request().prepare(None, control)?;
    if let Some(request) = snapshot.model().edit_request(EditOperation::Apply)? {
        let mut prepared = request.prepare(control)?;
        snapshot.model_mut().accept_edit(&mut prepared)?;
    }
    Ok(snapshot.model().document().source_snapshot().into())
}
#[cfg(test)]
mod tests;
