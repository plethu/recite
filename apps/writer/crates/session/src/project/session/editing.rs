//! Mutable views operate on the project's current model and recovery owner.
use super::{DocumentSessionMut, FileError};
use recite_writer_model::{
    ConditionValue, EffectAck, PreparedPreview, Preview, Workbench, WorkbenchError,
};

impl DocumentSessionMut<'_> {
    /// Every mutation queues recovery, including an operation that partially fails.
    pub fn edit(
        &mut self,
        action: impl FnOnce(&mut Workbench) -> Result<(), WorkbenchError>,
    ) -> Result<(), FileError> {
        let result = action(self.model).map_err(FileError::from);
        let recovery = self.files.queue_checkpoint(self.model);
        result.and(recovery)
    }
    pub fn save(&mut self) -> Result<(), FileError> {
        self.files.save_current(self.model)
    }
    pub fn prepare_external_edit(&mut self) -> Result<(), FileError> {
        self.files.require_idle_save()?;
        self.files.checkpoint(self.model)?;
        let mut write = self
            .files
            .prepare_write(super::super::save::ordered::Step::Current(
                self.model.document().source_snapshot(),
            ))?;
        write.execute()?;
        let result = self.files.publish_write(self.model, &mut write)?.wait();
        self.files.finish_save(self.model, result)
    }
    /// Trial state is transient and never queues document recovery writes.
    pub fn accept_preview(
        &mut self,
        preview: Box<PreparedPreview>,
    ) -> Result<Option<Preview>, Box<PreparedPreview>> {
        self.model.accept_preview(preview)
    }
    pub fn advance_preview(&mut self, choice: Option<usize>) -> Result<(), WorkbenchError> {
        self.model.advance_preview(choice)
    }
    pub fn answer_preview(&mut self, value: ConditionValue) -> Result<(), WorkbenchError> {
        self.model.answer_preview(value)
    }
    pub fn acknowledge_preview(&mut self, ack: EffectAck) -> Result<(), WorkbenchError> {
        self.model.acknowledge_preview(ack)
    }
}
