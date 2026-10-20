//! One save policy for both event loops and the blocking automation adapter.
mod accepting;
mod work;
use super::ordered::{Drafts, Scope, Step};
use crate::project::{FileError, ProjectFiles};
use recite_writer_model::Workbench;
use std::{collections::VecDeque, path::PathBuf};
pub use work::{Receipt, Work};

enum Pending {
    Edit(PathBuf),
    Write(PathBuf),
    Recovery(PathBuf),
    Manifest,
    DeclarationSource,
    DeclarationRecovery,
    DeclarationOutput,
    Retire,
}
pub enum Progress {
    Work(Work),
    Waiting,
    Finished(Result<(), FileError>),
}
/// Keep the owning project alive and harvest widget buffers before every receipt.
/// Dispatched writes must be reconciled even if the user requests cancellation.
pub struct SaveSequence {
    steps: VecDeque<Step>,
    queued: Option<(Work, Pending)>,
    after_retire: Option<(Work, Pending)>,
    pending: Option<Pending>,
    error: Option<FileError>,
    identify_file: bool,
    finished: bool,
}
impl SaveSequence {
    /// The frontend has already accepted its current Apply receipt, if required.
    /// Planning errors still produce a sequence so cleanup covers that accepted edit.
    pub fn accepted(files: &mut ProjectFiles, current: &Workbench, scope: Scope) -> Self {
        let identify_file = matches!(scope, Scope::All | Scope::Document(_));
        let checkpoint = files
            .recovery
            .begin_persist(files.recovery_value(current))
            .map(drop);
        let plan = checkpoint.and_then(|()| files.save_plan_accepted(current, scope));
        files.set_saving(true);
        let (steps, error) = match plan {
            Ok(steps) => (steps.into(), None),
            Err(error) => (VecDeque::new(), Some(error)),
        };
        Self {
            steps,
            queued: None,
            after_retire: None,
            pending: None,
            error,
            identify_file,
            finished: false,
        }
    }
    pub fn poll(&mut self, files: &mut ProjectFiles, current: &mut Workbench) -> Progress {
        if self.pending.is_some() {
            return Progress::Waiting;
        }
        loop {
            if let Some((work, pending)) = self.queued.take() {
                self.pending = Some(pending);
                return Progress::Work(work);
            }
            if self.finished {
                return Progress::Finished(Err(FileError::Selection));
            }
            if self.error.is_some() || self.steps.is_empty() {
                self.finished = true;
                files.set_saving(false);
                let result = self.error.take().map_or(Ok(()), Err);
                return Progress::Finished(files.finish_save(current, result));
            }
            if let Some(step) = self.steps.pop_front()
                && let Err(error) = self.prepare(files, step)
            {
                self.record(error);
            }
        }
    }
    /// A transport failure terminates only after the adapter has drained its work.
    pub fn failed(&mut self, error: FileError) {
        self.pending = None;
        self.record(error);
    }
    fn record(&mut self, error: FileError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }
    fn file_result<T>(&self, path: PathBuf, result: Result<T, FileError>) -> Result<T, FileError> {
        result.map_err(|error| {
            if self.identify_file {
                FileError::Saving {
                    path,
                    error: Box::new(error),
                }
            } else {
                error
            }
        })
    }
    fn prepare(&mut self, files: &mut ProjectFiles, step: Step) -> Result<(), FileError> {
        match step {
            Step::Manifest => {
                self.queued = files
                    .manifest_save()
                    .map(|request| (Work::Manifest(request), Pending::Manifest));
            }
            Step::Declarations { force } => {
                if let Some(session) = files
                    .declarations
                    .as_ref()
                    .filter(|session| force || session.dirty())
                {
                    self.queued = Some((
                        Work::DeclarationSource(Box::new(session.prepare_save()?)),
                        Pending::DeclarationSource,
                    ));
                }
            }
            step => self.prepare_document(files, step)?,
        }
        Ok(())
    }
    fn prepare_document(&mut self, files: &mut ProjectFiles, step: Step) -> Result<(), FileError> {
        let path = match &step {
            Step::Current(_) => files.current.clone(),
            Step::Retained { path, .. } => path.clone(),
            _ => return Err(FileError::Selection),
        };
        if let Some((path, request)) = self.file_result(path.clone(), files.retained_edit(&step))? {
            self.queued = Some((Work::Edit(request), Pending::Edit(path)));
        } else {
            let write = self.file_result(path.clone(), files.prepare_write(step))?;
            self.queued = Some((Work::Write(write), Pending::Write(path)));
        }
        Ok(())
    }
    fn retire(&mut self, work: Work, next: Option<(Work, Pending)>) {
        self.queued = Some((work, Pending::Retire));
        self.after_retire = next;
    }
}

#[cfg(test)]
mod tests;
