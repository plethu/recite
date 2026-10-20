//! External repairs prepare against the exact recovered field and accepted document.
use super::{
    PreparedSourceReplacement, SourceReplacementRequest, SourceValidation, View, Workbench,
    WorkbenchError,
};
use crate::{DraftSnapshot, EditError};
use recite_compiler::authoring::WorkControl;

pub struct SourceRepairRequest {
    replacement: SourceReplacementRequest,
}
pub struct PreparedSourceRepair {
    replacement: PreparedSourceReplacement,
}
impl Workbench {
    pub fn source_repair_request(&self) -> Result<SourceRepairRequest, WorkbenchError> {
        if self.view != View::Source {
            return Err(EditError::SourceRequired("open the recovered source draft first").into());
        }
        Ok(SourceRepairRequest {
            replacement: self.source_replacement_request(SourceValidation::NoNewErrors),
        })
    }
    pub fn accept_source_repair(
        &mut self,
        prepared: &mut PreparedSourceRepair,
    ) -> Result<(), WorkbenchError> {
        self.accept_source_replacement(&mut prepared.replacement)
    }
}
impl SourceRepairRequest {
    pub fn prepare(
        self,
        source: DraftSnapshot,
        control: &dyn WorkControl,
    ) -> Result<PreparedSourceRepair, WorkbenchError> {
        Ok(PreparedSourceRepair {
            replacement: self.replacement.prepare(source, control)?,
        })
    }
}
