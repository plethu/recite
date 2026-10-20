use super::*;
use recite_writer_session::project::save::sequence::{Progress, Receipt};

impl Writer {
    pub(crate) fn accept_save_work(&mut self, receipt: Receipt) {
        match self.work.operation.take() {
            Some(Operation::Applying(scope)) => self.accept_save_apply(scope, receipt),
            Some(Operation::RetiringApply(scope, result))
                if matches!(receipt, Receipt::Retired) =>
            {
                if let Err(error) = result {
                    self.finish_save_work(Err(error));
                } else {
                    self.begin_save_sequence(scope);
                }
            }
            Some(Operation::Saving(mut sequence)) => {
                if matches!(&receipt, Receipt::DeclarationOutput(_, Ok(()))) {
                    self.invalidate_declaration_output();
                }
                let Some((files, model)) = self.session.parts_mut() else {
                    self.status = FileError::ProjectRequired.to_string();
                    return;
                };
                sequence.accept(files, model, receipt);
                self.drive_save(sequence);
            }
            _ => self.finish_save_work(Err(FileError::Selection)),
        }
    }
    fn accept_save_apply(&mut self, scope: Scope, receipt: Receipt) {
        let Receipt::Edit(result) = receipt else {
            self.finish_save_work(Err(FileError::Selection));
            return;
        };
        match result {
            Ok(mut prepared) => {
                let Some((files, model)) = self.session.parts_mut() else {
                    self.status = FileError::ProjectRequired.to_string();
                    return;
                };
                let accepted = model.accept_edit(&mut prepared).map_err(FileError::from);
                // The explicit save retries recovery; an older asynchronous failure
                // must not veto this rescue of the newly accepted source.
                let _ = files.queue_checkpoint(model);
                self.work.operation = Some(Operation::RetiringApply(scope, accepted));
                self.submit_save_work(Work::RetireEdit(prepared));
            }
            Err(error) => self.finish_save_work(Err(error.into())),
        }
    }
    pub(super) fn drive_save(&mut self, mut sequence: Box<SaveSequence>) {
        let Some((files, model)) = self.session.parts_mut() else {
            self.status = FileError::ProjectRequired.to_string();
            return;
        };
        match sequence.poll(files, model) {
            Progress::Work(work) => {
                self.work.operation = Some(Operation::Saving(sequence));
                self.submit_save_work(work);
            }
            Progress::Finished(result) => self.finish_save_work(result),
            Progress::Waiting => {
                self.work.operation = Some(Operation::Saving(sequence));
            }
        }
    }
}
