//! One owned cold-load request; rejected and replaced sessions retire on its worker.
pub mod loading;
use super::*;
pub use crate::map::Workspace;
use loading::Loaded;
use recite_compiler::authoring::{Interrupted, WorkControl};
use std::sync::atomic;

pub enum Job {
    Open(Arc<Request>),
    #[cfg(target_os = "linux")]
    Listen(ActivationInbox),
    Browse,
    Create {
        path: PathBuf,
        name: String,
    },
}
pub struct Request {
    pub(crate) host: crate::Host,
    pub(crate) options: LaunchOptions,
    pub(crate) cancel: CancellationToken,
    pub(crate) forwarded: Option<Arc<atomic::AtomicBool>>,
}
pub struct Opening {
    pub(crate) request: Arc<Request>,
    pub(crate) origin: Option<recite_writer_model::NavigationRequest>,
    pub(crate) reply: Option<mpsc::Sender<Result<(), String>>>,
}
impl WorkControl for Request {
    fn checkpoint(&self) -> Result<(), Interrupted> {
        self.cancel.checkpoint()?;
        if self
            .forwarded
            .as_ref()
            .is_some_and(|cancelled| cancelled.load(atomic::Ordering::Acquire))
        {
            return Err(Interrupted);
        }
        Ok(())
    }
}
/// A finished load and the channel that retires whatever the shell does not install.
pub struct Opened {
    pub request: Arc<Request>,
    pub result: Result<Loaded, String>,
    pub retire: mpsc::SyncSender<Option<Box<Writer>>>,
}

pub enum Output {
    Opened(Opened),
    Chosen(Option<PathBuf>),
    Created(Result<PathBuf, String>),
    #[cfg(target_os = "linux")]
    Activation(Box<recite_writer_session::activation::IncomingRoute>),
}
impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectOutput").finish_non_exhaustive()
    }
}
impl Shell {
    pub fn accept_open(&mut self, output: Opened) {
        let current = self
            .opening
            .as_ref()
            .is_some_and(|pending| Arc::ptr_eq(&pending.request, &output.request));
        if !current {
            let _ = output
                .retire
                .send(output.result.ok().map(|loaded| loaded.writer));
            return;
        }
        let Some(pending) = self.opening.take() else {
            let _ = output
                .retire
                .send(output.result.ok().map(|loaded| loaded.writer));
            return;
        };
        let valid = output.request.checkpoint().is_ok()
            && self.can_replace()
            && match (&pending.origin, &self.writer) {
                (Some(origin), Some(writer)) => origin.is_current(writer.session.model()),
                (None, None) => true,
                _ => false,
            };
        let failed = valid && output.result.is_err();
        let (retired, result) = match output.result {
            Ok(Loaded { mut writer, location }) if valid => {
                writer.configure(self.preferences.config.clone());
                let view = writer.location.view.unwrap_or(self.preferences.config.writer.view);
                writer.map.workspace = Workspace::configured(view, self.preferences.config.writer.presentation.split());
                if writer.map.workspace.has_map() {
                    writer.invalidate_map();
                }
                let previous = self.writer.replace(*writer).map(Box::new);
                self.show_welcome = false;
                self.settings_open = false;
                self.status.clear();
                (previous, location)
            }
            Ok(loaded) => (Some(loaded.writer), Err("Project opening was cancelled or the current document changed. Your current work remains open.".into())),
            Err(error) => (None, Err(error)),
        };
        if let Err(error) = &result {
            if failed {
                self.project_failed(super::welcome::ProjectAction::Open, error.clone());
            } else {
                self.status = error.clone();
            }
        }
        let _ = output.retire.send(retired);
        if result.is_ok()
            && let Some(writer) = &mut self.writer
            && writer.attach_linked_editor_reply(
                pending.reply.clone(),
                pending.request.forwarded.clone(),
            )
        {
            return;
        }
        if let Some(reply) = pending.reply {
            let _ = reply.send(result);
        }
    }
}

impl Shell {
    /// Connect the project worker and start any launch request waiting for it.
    pub fn connect_project_worker(&mut self, sender: UnboundedSender<Job>) {
        self.sender = Some(sender);
        #[cfg(target_os = "linux")]
        if let Some(inbox) = self.inbox.take()
            && let Err(error) = self.send(Job::Listen(inbox))
        {
            self.status = error;
        }
        if let Some(options) = self.initial.take()
            && let Err(error) = self.start_open(options, None, None)
        {
            self.status = error;
        }
    }
    /// Apply one project worker result.
    pub fn accept_project_output(&mut self, output: Output) {
        match output {
            Output::Opened(output) => self.accept_open(output),
            Output::Chosen(path) => {
                self.auxiliary = false;
                if let Some(path) = path {
                    self.clear_project_problem();
                    self.path = path.display().to_string();
                    self.native_path = Some(path);
                }
            }
            Output::Created(result) => {
                self.auxiliary = false;
                match result {
                    Ok(path) => {
                        let options = LaunchOptions {
                            intent: Startup::Writer {
                                project: Some(path),
                                route: None,
                            },
                            document: None,
                            editor: None,
                        };
                        if let Err(error) = self.start_open(options, None, None) {
                            self.status = error;
                        }
                    }
                    Err(error) => self.project_failed(super::welcome::ProjectAction::Create, error),
                }
            }
            #[cfg(target_os = "linux")]
            Output::Activation(request) => self.accept_activation(*request),
        }
    }
}
