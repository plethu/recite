//! Native event-loop scheduling around Recite's shared save policy.
mod accepting;
mod worker;
use super::{Phase, Writer, feedback};
use recite_writer_model::EditOperation;
use recite_writer_session::{
    FileError,
    project::save::{
        ordered::Scope,
        sequence::{SaveSequence, Work},
    },
};
use tokio::sync::mpsc::UnboundedSender;
pub(super) use worker::worker;

enum Operation {
    Applying(Scope),
    RetiringApply(Scope, Result<(), FileError>),
    Saving(Box<SaveSequence>),
}
pub(super) struct EditorWork {
    pub(super) sender: Option<UnboundedSender<Work>>,
    operation: Option<Operation>,
    close_after: bool,
}
impl EditorWork {
    pub fn new() -> Self {
        Self {
            sender: None,
            operation: None,
            close_after: false,
        }
    }
    pub fn busy(&self) -> bool {
        self.operation.is_some()
    }
    pub fn keep_editing(&mut self) {
        self.close_after = false;
    }
}
impl Writer {
    pub(super) fn save(&mut self) {
        self.start_save(Scope::Current, false);
    }
    pub(super) fn save_all(&mut self) {
        self.start_save(Scope::All, false);
    }
    pub(super) fn save_and_close(&mut self) {
        self.start_save(Scope::All, true);
    }
    fn start_save(&mut self, scope: Scope, close_after: bool) {
        if self.work.busy() {
            return;
        }
        let prepared = (|| {
            self.project.files().require_idle_save()?;
            if self.work.sender.is_none() {
                return Err(std::io::Error::other(
                    "The editor worker is unavailable. Reopen Writer to retry.",
                )
                .into());
            }
            let (files, model) = self.project.parts_mut();
            let request = if scope.applies_current(&files.current) {
                let request = model.edit_request(EditOperation::Apply)?;
                if request.is_none() {
                    model.apply()?;
                }
                request
            } else {
                None
            };
            files.set_saving(true);
            Ok(request)
        })();
        match prepared {
            Ok(Some(request)) => {
                self.work.close_after = close_after;
                self.work.operation = Some(Operation::Applying(scope));
                self.submit_save_work(Work::Edit(request));
            }
            Ok(None) => {
                self.work.close_after = close_after;
                self.begin_save_sequence(scope);
            }
            Err(error) => self.finish_save_work(Err(error)),
        }
    }
    fn begin_save_sequence(&mut self, scope: Scope) {
        let (files, model) = self.project.parts_mut();
        let sequence = SaveSequence::accepted(files, model, scope);
        self.drive_save(Box::new(sequence));
    }
    fn submit_save_work(&mut self, work: Work) {
        let sent = self
            .work
            .sender
            .as_ref()
            .is_some_and(|sender| sender.send(work).is_ok());
        if sent {
            self.status = "Saving… You can keep editing.".into();
        } else {
            self.work.operation = None;
            self.finish_save_work(Err(std::io::Error::other(
                "The save worker stopped. Keep Writer open and retry.",
            )
            .into()));
        }
    }
    fn finish_save_work(&mut self, result: Result<(), FileError>) {
        let (files, model) = self.project.parts_mut();
        files.set_saving(false);
        let _ = files.queue_checkpoint(model);
        let result = files.finish_save(model, result);
        self.reading.prepare(self.project.current().model());
        self.status = match &result {
            Ok(()) if self.project.dirty() => "Saved; newer edits remain open.".into(),
            Ok(()) => "Saved".into(),
            Err(error) => feedback::file(error),
        };
        let close_after = std::mem::take(&mut self.work.close_after);
        if close_after && result.is_ok() && !self.project.dirty() {
            self.close_with_recovery();
        } else if close_after {
            self.phase = Phase::ConfirmClose;
        }
    }
}

#[cfg(test)]
mod tests;
