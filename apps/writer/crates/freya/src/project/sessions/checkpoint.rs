//! Only a completed, exact outgoing checkpoint authorizes a prepared scene switch.
use super::super::{FileError, ProjectFiles};
use crate::recovery::{Recovery, worker::Parking};
use recite_writer_model::{CompletionSnapshot, Workbench};
use std::path::PathBuf;

struct Capture {
    path: PathBuf,
    source: CompletionSnapshot,
    recovery: Option<Recovery>,
    sequence: u64,
}
pub(crate) struct Checkpoint {
    captured: Capture,
    parking: Parking<Recovery>,
}
pub(crate) struct ReadyCheckpoint(Capture);
impl ProjectFiles {
    pub(crate) fn begin_navigation_checkpoint(
        &mut self,
        model: &Workbench,
    ) -> Result<Checkpoint, FileError> {
        let recovery = self.recovery_value(model);
        self.recovery.begin_persist(recovery.clone())?;
        let captured = Capture {
            path: self.current.clone(),
            source: model.document().completion_snapshot(),
            recovery,
            sequence: self.recovery.sequence(),
        };
        Ok(Checkpoint {
            captured,
            parking: self.recovery.begin_park(),
        })
    }
}
impl Checkpoint {
    pub(crate) fn wait(self) -> Result<ReadyCheckpoint, FileError> {
        self.parking.wait()?;
        Ok(ReadyCheckpoint(self.captured))
    }
}
impl ReadyCheckpoint {
    pub(super) fn is_current(&self, files: &ProjectFiles, model: &Workbench) -> bool {
        self.0.path == files.current
            && self.0.source.is_current(model.document())
            && self.0.recovery == files.recovery_value(model)
            && files.recovery.parked_at(self.0.sequence)
    }
}

#[cfg(test)]
mod tests;
