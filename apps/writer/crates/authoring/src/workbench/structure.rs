use super::{View, Workbench, WorkbenchError};
use crate::EditError;
impl Workbench {
    pub fn set_continuation(&mut self, destination: &str) -> Result<(), WorkbenchError> {
        self.require_applied()?;
        let block = self.selected_block()?.ok_or(EditError::Destination)?;
        self.document
            .set_continuation(self.document.revision(), &block, destination)?;
        self.refresh()
    }

    pub fn rename_beat(&mut self, name: &str) -> Result<(), WorkbenchError> {
        self.require_applied()?;
        let block = self.selected_block()?.ok_or(EditError::Destination)?;
        self.document
            .rename_beat(self.document.revision(), &block, name)?;
        self.inspect_block(name)
    }

    pub fn add_beat(&mut self) -> Result<(), WorkbenchError> {
        self.require_applied()?;
        let id = self.document.add_beat(self.document.revision())?;
        self.inspect_block(&id)
    }
    pub fn add_line(&mut self) -> Result<(), WorkbenchError> {
        self.require_applied()?;
        let block = self.selected_block()?.ok_or(EditError::Destination)?;
        let id = self.document.add_line(self.document.revision(), &block)?;
        self.select(View::Passage(id))
    }
}
