//! One owned cold-load request; rejected and replaced sessions retire on its worker.
mod loading;
use super::*;
use crate::map::Workspace;
use loading::{Loaded, load};
use recite_compiler::authoring::{Interrupted, WorkControl};
use std::sync::atomic;
use tokio::{sync::mpsc::UnboundedReceiver, task::JoinHandle};
use xilem::{
    ViewCtx,
    core::{MessageProxy, NoElement, View},
};

pub(super) enum Job {
    Open(Arc<Request>),
    #[cfg(target_os = "linux")]
    Listen(ActivationInbox),
    Browse,
    Create {
        path: PathBuf,
        name: String,
    },
}
pub(super) struct Request {
    pub options: LaunchOptions,
    pub cancel: CancellationToken,
    pub forwarded: Option<Arc<atomic::AtomicBool>>,
}
pub(super) struct Opening {
    pub request: Arc<Request>,
    pub origin: Option<recite_writer_model::NavigationRequest>,
    pub reply: Option<mpsc::Sender<Result<(), String>>>,
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
struct Opened {
    request: Arc<Request>,
    result: Result<Loaded, String>,
    retire: mpsc::SyncSender<Option<Box<Writer>>>,
}

enum Output {
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
pub(super) fn worker() -> impl View<App, (), ViewCtx, Element = NoElement> {
    xilem::view::worker(
        serve,
        |app: &mut App, sender| {
            app.sender = Some(sender);
            #[cfg(target_os = "linux")]
            if let Some(inbox) = app.inbox.take()
                && let Err(error) = app.send(Job::Listen(inbox))
            {
                app.status = error;
            }
            if let Some(options) = app.initial.take()
                && let Err(error) = app.start_open(options, None, None)
            {
                app.status = error;
            }
        },
        |app: &mut App, output| match output {
            Output::Opened(output) => app.accept_open(output),
            Output::Chosen(path) => {
                app.auxiliary = false;
                if let Some(path) = path {
                    app.clear_project_problem();
                    app.path = path.display().to_string();
                    app.native_path = Some(path);
                }
            }
            Output::Created(result) => {
                app.auxiliary = false;
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
                        if let Err(error) = app.start_open(options, None, None) {
                            app.status = error;
                        }
                    }
                    Err(error) => app.project_failed(super::welcome::ProjectAction::Create, error),
                }
            }
            #[cfg(target_os = "linux")]
            Output::Activation(request) => app.accept_activation(*request),
        },
    )
}
async fn serve(proxy: MessageProxy<Output>, mut jobs: UnboundedReceiver<Job>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(30));
    let mut previous_open: Option<JoinHandle<()>> = None;
    #[cfg(target_os = "linux")]
    let mut inbox: Option<ActivationInbox> = None;
    loop {
        tokio::select! {
            job = jobs.recv() => {
                let Some(job) = job else { break; };
                match job {
                    #[cfg(target_os = "linux")]
                    Job::Listen(next) => inbox = Some(next),
                    Job::Open(request) => {
                        previous_open = Some(tokio::spawn(open_after(previous_open.take(), proxy.clone(), request)));
                    }
                    Job::Browse => { let selected = rfd::AsyncFileDialog::new().set_title("Choose project folder").pick_folder().await; let _ = proxy.message(Output::Chosen(selected.map(|file| file.path().to_owned()))); }
                    Job::Create { path, name } => {
                        let proxy = proxy.clone();
                        tokio::task::spawn_blocking(move || {
                            let result = recite_config::create_starter_project(&path, &name).map(|_| path).map_err(|error| error.to_string());
                            let _ = proxy.message(Output::Created(result));
                        });
                    }
                }
            }
            _ = interval.tick() => {
                #[cfg(target_os = "linux")]
                if let Some(request) = inbox.as_ref().and_then(ActivationInbox::try_next)
                    && proxy.message(Output::Activation(Box::new(request))).is_err() { break; }
            }
        }
    }
}

async fn open_after(
    previous: Option<JoinHandle<()>>,
    proxy: MessageProxy<Output>,
    request: Arc<Request>,
) {
    // Reopening waits for the previous owner's recovery lease to retire.
    // Await outside serve's loop so activation keeps receiving replies.
    if let Some(previous) = previous {
        let _ = previous.await;
    }
    let _ = tokio::task::spawn_blocking(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| load(&request)))
            .unwrap_or_else(|_| {
                Err("Project loading stopped unexpectedly. Reopen the project to retry.".into())
            });
        let (retire, retired) = mpsc::sync_channel(1);
        if proxy
            .message(Output::Opened(Opened {
                request,
                result,
                retire,
            }))
            .is_ok()
        {
            drop(retired.recv());
        }
    })
    .await;
}

impl App {
    fn accept_open(&mut self, output: Opened) {
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

#[cfg(test)]
mod tests;
