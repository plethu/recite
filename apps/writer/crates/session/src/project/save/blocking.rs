//! Blocking adapter for callers outside a native event loop; uses the same staged publication.
use super::*;
impl ProjectFiles {
    pub fn save_current(
        &mut self,
        current: &mut recite_writer_model::Workbench,
    ) -> Result<(), FileError> {
        self.save_synchronously(current, super::ordered::Scope::Current)
    }
    pub fn save_all(
        &mut self,
        current: &mut recite_writer_model::Workbench,
    ) -> Result<(), FileError> {
        self.save_synchronously(current, super::ordered::Scope::All)
    }
    pub fn save_document(
        &mut self,
        current: &mut recite_writer_model::Workbench,
        path: &Path,
    ) -> Result<(), FileError> {
        self.save_synchronously(current, super::ordered::Scope::Document(path.to_owned()))
    }
    pub(in crate::project) fn save_synchronously(
        &mut self,
        current: &mut recite_writer_model::Workbench,
        scope: super::ordered::Scope,
    ) -> Result<(), FileError> {
        self.require_idle_save()?;
        if scope.applies_current(&self.current) {
            let applied = current.apply().map_err(FileError::from);
            if let Err(error) = applied {
                let _ = self.queue_checkpoint(current);
                return self.finish_save(current, Err(error));
            }
        }
        let mut sequence = super::sequence::SaveSequence::accepted(self, current, scope);
        loop {
            match sequence.poll(self, current) {
                super::sequence::Progress::Work(work) => {
                    sequence.accept(self, current, work.execute());
                }
                super::sequence::Progress::Finished(result) => return result,
                super::sequence::Progress::Waiting => sequence.failed(FileError::SavePending),
            }
        }
    }
}
