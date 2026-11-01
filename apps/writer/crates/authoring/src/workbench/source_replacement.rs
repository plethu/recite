//! Full-source replacement retains document history and publishes a prepared valid selection.
use super::{FieldState, NavigationRequest, SourceValidation, Workbench, WorkbenchError};
use crate::{
    DraftSnapshot, EditError, EditOperation, RecoveredDraft, document::PreparedSourceEdit,
};
use recite_compiler::authoring::WorkControl;

pub struct SourceReplacementRequest {
    origin: NavigationRequest,
    validation: SourceValidation,
}
pub struct PreparedSourceReplacement {
    origin: NavigationRequest,
    source: PreparedSourceEdit,
    field: FieldState,
    recovery: RecoveredDraft,
    accepted: bool,
}
impl Workbench {
    pub fn source_replacement_request(
        &self,
        validation: SourceValidation,
    ) -> SourceReplacementRequest {
        SourceReplacementRequest {
            origin: self.navigation_request(),
            validation,
        }
    }
    /// Retain the receipt so displaced history, analysis and fields retire on the preparing worker.
    pub fn accept_source_replacement(
        &mut self,
        prepared: &mut PreparedSourceReplacement,
    ) -> Result<(), WorkbenchError> {
        if !prepared.is_current(self) {
            return Err(EditError::Stale.into());
        }
        self.document.accept_source_edit(&mut prepared.source)?;
        prepared.field.publish(self);
        prepared.accepted = true;
        Ok(())
    }
}
impl SourceReplacementRequest {
    pub fn prepare(
        self,
        source: DraftSnapshot,
        control: &dyn WorkControl,
    ) -> Result<PreparedSourceReplacement, WorkbenchError> {
        let prepared = self.origin.prepare(None, control)?;
        let (origin, mut model) = prepared.into_parts();
        let request = model
            .document
            .source_edit_request(source, EditOperation::Apply)
            .ok_or(EditError::Stale)?
            .requiring_validation(self.validation);
        let source = request.prepare(control)?;
        model.document = source.projection_document();
        model.refresh()?;
        model.source_validation = self.validation;
        let recovery = model.recovery();
        let (_, field) = FieldState::take(model);
        Ok(PreparedSourceReplacement {
            origin,
            source,
            field,
            recovery,
            accepted: false,
        })
    }
}
impl PreparedSourceReplacement {
    pub fn is_current(&self, model: &Workbench) -> bool {
        !self.accepted && self.origin.is_current(model) && self.source.is_current(&model.document)
    }
    pub fn recovery(&self) -> &RecoveredDraft {
        &self.recovery
    }
}
