//! An independently reaped editor opens only after accepted source and recovery are durable.
pub mod linked;
pub mod request;
use super::{Phase, Writer};
use recite_writer_session::startup::EditorCommand;
pub use request::Request;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

pub struct ExternalEditor {
    pub sender: Option<UnboundedSender<Arc<Request>>>,
    pending: Option<Arc<Request>>,
    failed: Option<Arc<Request>>,
    waiting: Option<linked::Link>,
}
impl Default for ExternalEditor {
    fn default() -> Self {
        Self::new()
    }
}
impl ExternalEditor {
    pub fn new() -> Self {
        Self {
            sender: None,
            pending: None,
            failed: None,
            waiting: None,
        }
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some() || self.waiting.is_some()
    }
    pub fn path(&self) -> Option<&std::path::Path> {
        self.pending.as_ref().map(|request| request.path())
    }
}
impl Writer {
    pub fn open_exported_source(&mut self) {
        let Some(artifact) = &self.repair.artifact else {
            return;
        };
        if self.external.busy() {
            self.status = "An editor launch is already in progress.".into();
            return;
        }
        self.launch_editor(Arc::new(Request::repair(artifact, self.editor.clone())));
    }
    pub fn open_editor(&mut self) {
        self.prepare_editor(self.editor.clone());
    }
    pub fn open_system_editor(&mut self) {
        if self.work.busy() || self.navigation.busy() || self.external.busy() {
            self.status = "Finish the current editor action before retrying the launcher.".into();
            return;
        }
        if let Some(failed) = &self.external.failed
            && failed.is_separate_file()
            && self
                .session
                .project()
                .is_some_and(|project| failed.is_current(project))
        {
            let request = Arc::new(failed.with_system_editor());
            self.launch_editor(request);
            return;
        }
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
        self.save_for_editor(Arc::new(Request::source(
            project.current().path().to_owned(),
            editor,
        )));
    }
    pub fn cancel_editor_launch(&mut self) {
        if self.work.preparing_editor() {
            self.work.keep_editing();
            self.status = "Editor launch cancelled; the accepted save will finish.".into();
        }
    }
    pub fn launch_editor(&mut self, request: Arc<Request>) {
        if self.phase != Phase::Writing
            || self
                .session
                .project()
                .is_none_or(|project| !request.is_current(project))
        {
            self.status = "Editor launch cancelled because its source context changed.".into();
            request.finish_link(Err(self.status.clone()));
            return;
        }
        if let Err(error) = request.check_link() {
            request.finish_link(Err(error.clone()));
            self.status = error;
            return;
        }
        let sent = self
            .external
            .sender
            .as_ref()
            .is_some_and(|sender| sender.send(Arc::clone(&request)).is_ok());
        if sent {
            self.external.failed = None;
            self.external.pending = Some(request);
            self.status =
                "Opening source in your editor… Writer drafts remain open and recoverable.".into();
        } else {
            self.status =
                "Could not open the editor: the launcher stopped. Your draft remains open.".into();
            request.finish_link(Err(self.status.clone()));
        }
    }
    pub fn accept_editor(&mut self, request: Arc<Request>, result: Option<Result<(), String>>) {
        if !self
            .external
            .pending
            .as_ref()
            .is_some_and(|pending| Arc::ptr_eq(pending, &request))
        {
            return;
        }
        let finished = result.is_some();
        let failed = result.as_ref().is_some_and(Result::is_err);
        let current = self
            .session
            .project()
            .is_some_and(|project| request.is_current(project));
        let document = self
            .session
            .project()
            .and_then(|project| request.path().strip_prefix(project.root()).ok())
            .unwrap_or(request.path())
            .display();
        self.status = match result {
            None => {
                format!("Editor launch requested for {document}; Writer drafts remain recoverable.")
            }
            Some(Ok(())) if current => format!(
                "Editor launcher finished for {document}. {}; Writer drafts remain recoverable.",
                request.next_step()
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
            if failed {
                self.external.failed = Some(request);
            }
            self.external.pending = None;
        }
    }
}
