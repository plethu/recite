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
#[derive(Default)]
enum AfterSave {
    #[default]
    Stay,
    Close,
    Build(Option<String>),
    Declarations,
    OpenEditor(std::sync::Arc<crate::external_editor::Request>),
}
pub(super) struct EditorWork {
    pub(super) sender: Option<UnboundedSender<Work>>,
    operation: Option<Operation>,
    after: AfterSave,
}
impl EditorWork {
    pub fn new() -> Self {
        Self {
            sender: None,
            operation: None,
            after: AfterSave::Stay,
        }
    }
    pub fn busy(&self) -> bool {
        self.operation.is_some()
    }
    pub fn keep_editing(&mut self) {
        self.after = AfterSave::Stay;
    }
    pub fn preparing_editor(&self) -> bool {
        matches!(self.after, AfterSave::OpenEditor(_))
    }
}
impl Writer {
    pub(super) fn save(&mut self) {
        self.start_save(Scope::Current, AfterSave::Stay);
    }
    pub(super) fn save_all(&mut self) {
        self.start_save(Scope::All, AfterSave::Stay);
    }
    pub(crate) fn save_declarations(&mut self) {
        self.start_save(Scope::Declarations, AfterSave::Declarations);
    }
    pub(super) fn save_and_close(&mut self) {
        self.start_save(Scope::All, AfterSave::Close);
    }
    pub(crate) fn save_and_build(&mut self) {
        if self.phase != Phase::Writing {
            return;
        }
        let selected = self
            .session
            .project()
            .and_then(|project| project.files().builds.selected.clone());
        self.start_save(Scope::All, AfterSave::Build(selected));
    }
    pub(super) fn save_for_editor(
        &mut self,
        request: std::sync::Arc<crate::external_editor::Request>,
    ) {
        self.start_save(Scope::AcceptedCurrent, AfterSave::OpenEditor(request));
    }
    fn start_save(&mut self, scope: Scope, after: AfterSave) {
        if let Some(document) = self.source_repair_requirement(&scope) {
            self.status = format!(
                "Repair and import the source draft in {} before saving it. No project files changed.",
                document.display()
            );
            return;
        }
        if self.navigation.busy() {
            if !matches!(&after, AfterSave::Stay) || !self.defer_save_until_apply(&scope) {
                self.status = "Finish the current editor action before saving.".into();
            }
            return;
        }
        if self.work.busy() {
            return;
        }
        let prepared = (|| {
            self.session
                .project()
                .ok_or(FileError::ProjectRequired)?
                .files()
                .require_idle_save()?;
            if self.work.sender.is_none() {
                return Err(std::io::Error::other(
                    "The editor worker is unavailable. Reopen Writer to retry.",
                )
                .into());
            }
            let rules = self.rules.as_ref().map(|reply| reply.projection().clone());
            let (files, model) = self.session.parts_mut().ok_or(FileError::ProjectRequired)?;
            let request = if scope.applies_current(&files.current) {
                let request = model.edit_request(EditOperation::Apply)?;
                if request.is_none() {
                    model.apply()?;
                }
                request.map(|request| {
                    if let Some(rules) = rules {
                        request.with_rules(rules)
                    } else {
                        request
                    }
                })
            } else {
                None
            };
            files.set_saving(true);
            Ok(request)
        })();
        match prepared {
            Ok(Some(request)) => {
                self.work.after = after;
                self.work.operation = Some(Operation::Applying(scope));
                self.submit_save_work(Work::Edit(request));
            }
            Ok(None) => {
                self.work.after = after;
                self.begin_save_sequence(scope);
            }
            Err(error) => self.finish_save_work(Err(error)),
        }
    }
    fn begin_save_sequence(&mut self, scope: Scope) {
        let Some((files, model)) = self.session.parts_mut() else {
            self.status = FileError::ProjectRequired.to_string();
            return;
        };
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
            self.status = if self.rules.is_some() {
                "Saving… Reply controls will reopen when it finishes."
            } else {
                "Saving… You can keep editing."
            }
            .into();
        } else {
            self.work.operation = None;
            self.finish_save_work(Err(std::io::Error::other(
                "The save worker stopped. Keep Writer open and retry.",
            )
            .into()));
        }
    }
    fn finish_save_work(&mut self, result: Result<(), FileError>) {
        let Some((files, model)) = self.session.parts_mut() else {
            self.status = FileError::ProjectRequired.to_string();
            return;
        };
        files.set_saving(false);
        let _ = files.queue_checkpoint(model);
        let result = files.finish_save(model, result);
        self.reading.prepare(self.session.model());
        self.status = match &result {
            Ok(()) if self.session.dirty() => "Saved; newer edits remain open.".into(),
            Ok(()) => "Saved".into(),
            Err(error) => feedback::file(error),
        };
        match std::mem::take(&mut self.work.after) {
            AfterSave::Close if result.is_ok() && !self.session.dirty() => {
                self.close_with_recovery()
            }
            AfterSave::Close => self.phase = Phase::ConfirmClose,
            AfterSave::OpenEditor(request) if result.is_ok() => self.launch_editor(request),
            AfterSave::Build(asset) if result.is_ok() => self.build_selected(asset),
            AfterSave::Declarations if result.is_ok() => self.refresh_declaration_rows(),
            AfterSave::Declarations => {
                if let Err(error) = &result {
                    self.schema_feedback(error.to_string());
                }
            }
            AfterSave::Stay | AfterSave::OpenEditor(_) | AfterSave::Build(_) => {}
        }
        if self.phase == Phase::Writing {
            self.refresh_rules();
            self.invalidate_map();
            self.refresh_map_if_needed();
        }
    }
}

#[cfg(test)]
mod tests;
