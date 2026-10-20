//! External repairs prepare against the exact recovered field and accepted document.
use super::{NavigationRequest, PreparedEdit, PreparedNavigation, View, Workbench, WorkbenchError};
use crate::{DraftSnapshot, EditError, EditOperation};
use recite_compiler::authoring::WorkControl;

pub struct SourceRepairRequest {
    origin: NavigationRequest,
}
pub struct PreparedSourceRepair {
    origin: PreparedNavigation,
    edit: PreparedEdit,
    draft: DraftSnapshot,
    accepted: bool,
}
impl Workbench {
    pub fn source_repair_request(&self) -> Result<SourceRepairRequest, WorkbenchError> {
        if self.view != View::Source {
            return Err(EditError::SourceRequired("open the recovered source draft first").into());
        }
        Ok(SourceRepairRequest {
            origin: self.navigation_request(),
        })
    }
    pub fn accept_source_repair(
        &mut self,
        prepared: &mut PreparedSourceRepair,
    ) -> Result<(), WorkbenchError> {
        if prepared.accepted || !prepared.origin.is_current(self) {
            return Err(EditError::Stale.into());
        }
        let previous = self.draft.clone();
        let validation = self.source_validation;
        self.stage_repaired_source(prepared.draft.clone())?;
        if let Err(error) = self.accept_edit(&mut prepared.edit) {
            self.set_draft_snapshot(previous);
            self.source_validation = validation;
            return Err(error);
        }
        prepared.accepted = true;
        Ok(())
    }
}
impl SourceRepairRequest {
    pub fn prepare(
        self,
        source: DraftSnapshot,
        control: &dyn WorkControl,
    ) -> Result<PreparedSourceRepair, WorkbenchError> {
        let mut origin = self.origin.prepare(None, control)?;
        let model = origin.model_mut();
        model.stage_repaired_source(source)?;
        let draft = model.draft_snapshot().clone();
        let edit = model
            .edit_request(EditOperation::Apply)?
            .ok_or(EditError::Stale)?;
        let edit = edit.prepare(control)?;
        Ok(PreparedSourceRepair {
            origin,
            edit,
            draft,
            accepted: false,
        })
    }
}
