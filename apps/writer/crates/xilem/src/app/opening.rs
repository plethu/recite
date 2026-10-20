//! One owned cold-load request; rejected and replaced sessions retire on its worker.
use super::*;
use recite_compiler::authoring::{Interrupted, WorkControl};
use tokio::sync::mpsc::UnboundedReceiver;
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
    pub forwarded: Option<Arc<std::sync::atomic::AtomicBool>>,
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
            .is_some_and(|cancelled| cancelled.load(std::sync::atomic::Ordering::Acquire))
        {
            return Err(Interrupted);
        }
        Ok(())
    }
}
struct Opened {
    request: Arc<Request>,
    result: Result<Box<Writer>, String>,
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
                    Err(error) => app.status = error,
                }
            }
            #[cfg(target_os = "linux")]
            Output::Activation(request) => app.accept_activation(*request),
        },
    )
}
async fn serve(proxy: MessageProxy<Output>, mut jobs: UnboundedReceiver<Job>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(30));
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
                        let proxy = proxy.clone();
                        // Keep polling activation while a cancellable compiler phase runs.
                        tokio::task::spawn_blocking(move || {
                            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| load(&request))).unwrap_or_else(|_| Err("Project loading stopped unexpectedly. Reopen the project to retry.".into()));
                            let (retire, retired) = mpsc::sync_channel(1);
                            if proxy.message(Output::Opened(Opened { request, result, retire })).is_ok() { drop(retired.recv()); }
                        });
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
fn load(request: &Request) -> Result<Box<Writer>, String> {
    request.checkpoint().map_err(|e| e.to_string())?;
    let location = request.options.location()?;
    let session = match &request.options.intent {
        Startup::Writer {
            project: Some(project),
            ..
        } => {
            let document = request.options.document.clone().or_else(|| {
                location
                    .as_ref()
                    .filter(|location| !location.document.is_empty())
                    .map(|location| location.document.clone().into())
            });
            Session::Project(Box::new(
                recite_writer_session::ProjectSession::open_with_control(
                    project,
                    document.as_deref(),
                    request,
                )
                .map_err(|e| e.to_string())?,
            ))
        }
        Startup::Examples { .. } => Session::examples().map_err(|e| e.to_string())?,
        _ => return Err("Choose a project or a temporary example.".into()),
    };
    let mut writer = Writer::from_session(session, request.options.editor.clone());
    if let Some(location) = location
        && let Err(error) = writer.follow_location(&location)
    {
        if writer.session.dirty() {
            writer.status =
                format!("Recovered your draft; the linked location could not open: {error}");
        } else {
            return Err(error);
        }
    }
    request.checkpoint().map_err(|e| e.to_string())?;
    Ok(Box::new(writer))
}
impl App {
    fn accept_open(&mut self, output: Opened) {
        let current = self
            .opening
            .as_ref()
            .is_some_and(|pending| Arc::ptr_eq(&pending.request, &output.request));
        if !current {
            let _ = output.retire.send(output.result.ok());
            return;
        }
        let Some(pending) = self.opening.take() else {
            let _ = output.retire.send(output.result.ok());
            return;
        };
        let valid = output.request.checkpoint().is_ok()
            && self.can_replace()
            && match (&pending.origin, &self.writer) {
                (Some(origin), Some(writer)) => origin.is_current(writer.session.model()),
                (None, None) => true,
                _ => false,
            };
        let (retired, result) = match output.result {
            Ok(writer) if valid => {
                let previous = self.writer.replace(*writer).map(Box::new);
                self.show_welcome = false;
                self.status.clear();
                (previous, Ok(()))
            }
            Ok(writer) => (Some(writer), Err("Project opening was cancelled or the current document changed. Your current work remains open.".into())),
            Err(error) => (None, Err(error)),
        };
        if let Err(error) = &result {
            self.status = error.clone();
        }
        let _ = output.retire.send(retired);
        if let Some(reply) = pending.reply {
            let _ = reply.send(result);
        }
    }
}

#[cfg(test)]
mod tests;
