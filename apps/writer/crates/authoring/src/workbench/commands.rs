//! Structural commands prepare against an applied snapshot and publish one live history step.
use super::{FieldState, NavigationRequest, View, Workbench, WorkbenchError};
use crate::{
    EditError, EditOperation,
    document::{PreparedSourceEdit, SourceEditRequest},
};
use recite_compiler::authoring::{AuthoringError, WorkControl};

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RouteOwner {
    Reply(String),
    Beat(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StructuralCommand {
    AddBeat,
    AddLine,
    AddReply,
    Destination {
        owner: RouteOwner,
        destination: String,
    },
    RenameBeat(String),
}
pub struct StructuralRequest {
    origin: NavigationRequest,
    source: SourceEditRequest,
    command: StructuralCommand,
}
pub struct PreparedStructuralEdit {
    origin: NavigationRequest,
    source: PreparedSourceEdit,
    selection: FieldState,
    accepted: bool,
}
impl Workbench {
    pub fn structural_request(
        &self,
        command: StructuralCommand,
    ) -> Result<StructuralRequest, WorkbenchError> {
        self.require_applied()?;
        self.document.check_revision(self.draft_revision)?;
        let source = self
            .document
            .source_edit_request(self.draft_snapshot().clone(), EditOperation::Apply)
            .ok_or(EditError::Stale)?;
        Ok(StructuralRequest {
            origin: self.navigation_request(),
            source,
            command,
        })
    }
    /// Retain the receipt for worker-side destruction of displaced projections and history.
    pub fn accept_structural_edit(
        &mut self,
        prepared: &mut PreparedStructuralEdit,
    ) -> Result<(), WorkbenchError> {
        if prepared.accepted || !prepared.origin.is_current(self) {
            return Err(EditError::Stale.into());
        }
        self.document.accept_source_edit(&mut prepared.source)?;
        prepared.selection.publish(self);
        prepared.accepted = true;
        Ok(())
    }
}
impl StructuralRequest {
    pub fn prepare(
        self,
        control: &dyn WorkControl,
    ) -> Result<PreparedStructuralEdit, WorkbenchError> {
        let prepared = self.origin.prepare(None, control)?;
        let (origin, mut model) = prepared.into_parts();
        // Existing structural methods own source preservation, validation and stable IDs.
        // Their compiler phases finish before cancellation can reject publication.
        self.command.apply(&mut model)?;
        control
            .checkpoint()
            .map_err(AuthoringError::from)
            .map_err(EditError::from)?;
        let (document, selection) = FieldState::take(model);
        let source = self.source.prepare_candidate(document)?;
        Ok(PreparedStructuralEdit {
            origin,
            source,
            selection,
            accepted: false,
        })
    }
}
impl StructuralCommand {
    fn apply(self, model: &mut Workbench) -> Result<(), WorkbenchError> {
        match self {
            Self::AddBeat => model.add_beat(),
            Self::AddLine => model.add_line(),
            Self::AddReply => model.add_choice(),
            Self::RenameBeat(name) => model.rename_beat(&name),
            Self::Destination {
                owner: RouteOwner::Reply(id),
                destination,
            } => {
                model.select(View::Passage(id.clone()))?;
                model
                    .document
                    .set_destination(model.document.revision(), &id, &destination)?;
                model.refresh()
            }
            Self::Destination {
                owner: RouteOwner::Beat(id),
                destination,
            } => {
                model.select(View::Block(id))?;
                model.set_continuation(&destination)
            }
        }
    }
}
