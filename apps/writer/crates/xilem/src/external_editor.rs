//! An independently reaped editor opens only after accepted source and recovery are durable.
#[cfg(test)]
mod tests;
mod worker;
use super::{Phase, Writer};
use recite_writer_session::startup::EditorCommand;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;
pub(super) use worker::worker;

pub(super) struct Request {
    path: PathBuf,
    editor: Option<EditorCommand>,
}
pub(super) struct ExternalEditor {
    pub sender: Option<UnboundedSender<Arc<Request>>>,
    pending: Option<Arc<Request>>,
}
impl ExternalEditor {
    pub fn new() -> Self {
        Self {
            sender: None,
            pending: None,
        }
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn path(&self) -> Option<&std::path::Path> {
        self.pending.as_ref().map(|request| request.path.as_path())
    }
}
impl Writer {
    pub(super) fn open_editor(&mut self) {
        self.prepare_editor(self.editor.clone());
    }
    pub(super) fn open_system_editor(&mut self) {
        self.prepare_editor(None);
    }
    fn prepare_editor(&mut self, editor: Option<EditorCommand>) {
        if self.work.busy() || self.navigation.busy() || self.external.busy() {
            self.status =
                "An editor or save request is already in progress. You can keep editing.".into();
            return;
        }
        let Some(project) = self.session.project() else {
            self.status = "Open a project to edit its source in your editor.".into();
            return;
        };
        if self.external.sender.is_none() {
            self.status = "The editor launcher is unavailable. Reopen Writer to retry.".into();
            return;
        }
        self.save_for_editor(Arc::new(Request {
            path: project.current().path().to_owned(),
            editor,
        }));
    }
    pub(super) fn cancel_editor_launch(&mut self) {
        if self.work.preparing_editor() {
            self.work.keep_editing();
            self.status = "Editor launch cancelled; the accepted save will finish.".into();
        }
    }
    pub(super) fn launch_editor(&mut self, request: Arc<Request>) {
        if self.phase != Phase::Writing
            || self
                .session
                .project()
                .is_none_or(|project| project.current().path() != request.path)
        {
            self.status =
                "Saved accepted source; editor launch cancelled because the document changed."
                    .into();
            return;
        }
        let sent = self
            .external
            .sender
            .as_ref()
            .is_some_and(|sender| sender.send(Arc::clone(&request)).is_ok());
        if sent {
            self.external.pending = Some(request);
            self.status =
                "Opening source in your editor… Writer drafts remain open and recoverable.".into();
        } else {
            self.status =
                "Could not open the editor: the launcher stopped. Your draft remains open.".into();
        }
    }
    fn accept_editor(&mut self, request: Arc<Request>, result: Option<Result<(), String>>) {
        if !self
            .external
            .pending
            .as_ref()
            .is_some_and(|pending| Arc::ptr_eq(pending, &request))
        {
            return;
        }
        let finished = result.is_some();
        let current = self
            .session
            .project()
            .is_some_and(|project| project.current().path() == request.path);
        let document = self
            .session
            .project()
            .and_then(|project| request.path.strip_prefix(project.root()).ok())
            .unwrap_or(&request.path)
            .display();
        self.status = match result {
            None => {
                format!("Editor launch requested for {document}; Writer drafts remain recoverable.")
            }
            Some(Ok(())) if current => format!(
                "Editor launcher finished for {document}. Reload to inspect external changes; Writer drafts remain recoverable."
            ),
            Some(Ok(())) => format!(
                "Editor launcher finished for {document}. Return to that document to inspect external changes."
            ),
            Some(Err(error)) => match &request.editor {
                Some(editor) => format!(
                    "Could not start the configured editor ‘{}’ for {document}. Check that it is installed, or choose Use system default. Writer drafts remain open. Details: {error}",
                    editor.program.to_string_lossy()
                ),
                None => format!(
                    "Could not open {document} with the system's preferred application. Check its file association and retry. Writer drafts remain open. Details: {error}"
                ),
            },
        };
        if finished {
            self.external.pending = None;
        }
    }
}
