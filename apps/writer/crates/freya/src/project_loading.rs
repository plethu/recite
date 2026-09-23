//! One project load at a time; cancelled or superseded work never replaces edits.
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
    project::{FileError, ProjectFiles},
};
use freya::prelude::*;
use std::{path::PathBuf, sync::mpsc, thread::JoinHandle};

pub(crate) fn can_switch_project(writer: Writer) -> bool {
    !writer.localisation.peek().dirty()
        && !writer.localisation.peek().modal_open()
        && !*writer.settings_open.peek()
        && !crate::design::modal_open()
        && writer.command_search.mode.peek().is_none()
        && writer.buffers.can_leave(writer.files.peek().as_ref())
}

#[cfg(target_os = "linux")]
fn receive_activation(
    writer: Writer,
    files: State<Option<ProjectFiles>>,
    job: State<Option<LoadJob>>,
    request: crate::activation::IncomingRoute,
    ready: bool,
) {
    let reply = request.reply.clone();
    let result = (|| {
        if request.cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err("Writer activation was cancelled".to_string());
        }
        request
            .route
            .as_deref()
            .map(crate::navigation::project_from_route)
            .transpose()?;
        // The socket worker canonicalised the project intent before queueing it.
        let target = request.project.as_ref();
        if job.peek().is_some() {
            return Err("A project is still opening. Cancel it or wait for it to finish.".into());
        }
        if let Some(target) = target
            && files
                .peek()
                .as_ref()
                .is_none_or(|files| files.root() != target)
        {
            if !can_switch_project(writer) {
                return Err(
                    "Save changes and apply or discard drafts before opening another project."
                        .into(),
                );
            }
            return start_forwarded(job, target.clone(), writer.buffers, request).map(|()| true);
        }
        crate::navigation::activation::receive(
            writer,
            request.route.as_deref(),
            &request.cancelled,
            ready,
        )
        .map(|()| false)
    })();
    match result {
        Ok(false) => {
            let _ = reply.send(Ok(()));
        }
        Ok(true) => {}
        Err(error) => {
            let _ = reply.send(Err(error.clone()));
            let mut message = writer.message;
            message.error(error);
        }
    }
}

pub(crate) struct LoadJob {
    result: mpsc::Receiver<Result<(ProjectFiles, recite_writer_model::Workbench), FileError>>,
    worker: Option<JoinHandle<()>>,
    cancelled: bool,
    source: Option<std::sync::Arc<str>>,
    #[cfg(target_os = "linux")]
    activation: Option<crate::activation::IncomingRoute>,
}
impl Drop for LoadJob {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
pub(crate) fn start(
    job: State<Option<LoadJob>>,
    path: PathBuf,
    mut message: crate::feedback::Feedback,
    buffers: crate::buffers::Buffers,
) {
    if let Err(error) = start_inner(job, path, buffers, None) {
        message.info(error);
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn start_forwarded(
    job: State<Option<LoadJob>>,
    path: PathBuf,
    buffers: crate::buffers::Buffers,
    activation: crate::activation::IncomingRoute,
) -> Result<(), String> {
    start_inner(job, path, buffers, Some(activation))
}

fn start_inner(
    mut job: State<Option<LoadJob>>,
    path: PathBuf,
    buffers: crate::buffers::Buffers,
    #[cfg(target_os = "linux")] activation: Option<crate::activation::IncomingRoute>,
    #[cfg(not(target_os = "linux"))] _activation: Option<()>,
) -> Result<(), String> {
    if job.peek().is_some() {
        return Err("A project is still opening. Cancel it or wait for it to finish.".into());
    }
    let (sender, result) = mpsc::sync_channel(1);
    match std::thread::Builder::new()
        .name("recite-project-open".into())
        .spawn(move || {
            let loaded = ProjectFiles::open(&path).and_then(|mut project| {
                let workbench = project.workbench()?;
                Ok((project, workbench))
            });
            let _ = sender.send(loaded);
        }) {
        Ok(worker) => {
            job.set(Some(LoadJob {
                result,
                worker: Some(worker),
                cancelled: false,
                source: buffers
                    .model
                    .peek()
                    .as_ref()
                    .ok()
                    .map(|m| m.document().source_snapshot()),
                #[cfg(target_os = "linux")]
                activation,
            }));
            Ok(())
        }
        Err(error) => Err(format!("Could not start opening the project: {error}")),
    }
}
#[derive(Clone)]
pub(crate) struct Loading {
    pub writer: Writer,
    pub files: State<Option<ProjectFiles>>,
    pub job: State<Option<LoadJob>>,
    pub panel: State<bool>,
}
impl PartialEq for Loading {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark && self.job == other.job
    }
}
impl Component for Loading {
    fn render(&self) -> impl IntoElement {
        let mut tick = freya::sdk::use_timeout(|| std::time::Duration::from_millis(50));
        #[cfg(target_os = "linux")]
        let inbox = use_try_consume::<crate::ActivationInbox>();
        #[cfg(target_os = "linux")]
        let ready = use_try_consume::<crate::navigation::NavigationReady>();
        let mut job = self.job;
        let mut files = self.files;
        let mut panel = self.panel;
        let mut message = self.writer.message;
        if tick.elapsed() {
            tick.reset();
            #[cfg(target_os = "linux")]
            if let Some(inbox) = &inbox {
                while let Some(request) = inbox.try_next() {
                    receive_activation(
                        self.writer,
                        files,
                        job,
                        request,
                        ready.is_some_and(|ready| *ready.0.peek()),
                    );
                }
            }
            let loaded = job
                .peek()
                .as_ref()
                .and_then(|job| match job.result.try_recv() {
                    Ok(result) => Some((job.cancelled, job.source.clone(), result)),
                    Err(mpsc::TryRecvError::Empty) => None,
                    Err(mpsc::TryRecvError::Disconnected) => Some((
                        job.cancelled,
                        job.source.clone(),
                        Err(FileError::Io(std::io::Error::other(
                            "Project loading stopped unexpectedly",
                        ))),
                    )),
                });
            if let Some((cancelled, source, loaded)) = loaded {
                let current = self
                    .writer
                    .buffers
                    .model
                    .peek()
                    .as_ref()
                    .ok()
                    .map(|m| m.document().source_snapshot());
                #[cfg(target_os = "linux")]
                let activation = job.write().take().and_then(|mut job| job.activation.take());
                #[cfg(not(target_os = "linux"))]
                job.set(None);
                #[cfg(target_os = "linux")]
                let activation_cancelled = activation.as_ref().is_some_and(|request| {
                    request.cancelled.load(std::sync::atomic::Ordering::Acquire)
                });
                #[cfg(not(target_os = "linux"))]
                let activation_cancelled = false;
                if cancelled || activation_cancelled {
                    message.info("Project opening cancelled.".into());
                    #[cfg(target_os = "linux")]
                    if let Some(request) = activation {
                        let _ = request.reply.send(Err("Project opening cancelled".into()));
                    }
                } else if source != current || !can_switch_project(self.writer) {
                    message.error("The current document changed while loading. Save it before opening another project.".into());
                    #[cfg(target_os = "linux")]
                    if let Some(request) = activation {
                        let _ = request
                            .reply
                            .send(Err("The current document changed while loading".into()));
                    }
                } else {
                    match loaded {
                        Ok((project, workbench)) => {
                            let recovered = project.has_recovery();
                            let opened_root = project.root().display().to_string();
                            let reset = {
                                let mut state = self.writer.localisation;
                                let mut localisation = state.write();
                                localisation.install(None).map(|()| {
                                    *localisation = crate::localisation::Localisation::default();
                                })
                            };
                            if let Err(error) = reset {
                                message.error(error.clone());
                                #[cfg(target_os = "linux")]
                                if let Some(request) = activation {
                                    let _ = request.reply.send(Err(error));
                                }
                                return rect();
                            }
                            self.writer.buffers.install(workbench, self.writer.dark);
                            files.set(Some(project));
                            if let Err(error) = self.writer.scene_opened() {
                                let error = format!(
                                    "Project {opened_root} opened, but its initial scene could not open: {error}"
                                );
                                message.error(error.clone());
                                #[cfg(target_os = "linux")]
                                if let Some(request) = activation {
                                    let _ = request.reply.send(Err(error));
                                }
                                return rect();
                            }
                            panel.set(false);
                            message.info(
                                if recovered {
                                    "Recovered your previous session."
                                } else {
                                    "Project opened."
                                }
                                .into(),
                            );
                            #[cfg(target_os = "linux")]
                            if let Some(request) = activation {
                                let result = crate::navigation::activation::receive(
                                    self.writer,
                                    request.route.as_deref(),
                                    &request.cancelled,
                                    true,
                                ).map_err(|error| format!(
                                    "Project {opened_root} opened, but the target location could not open: {error}"
                                ));
                                if let Err(error) = &result {
                                    message.error(error.clone());
                                }
                                let _ = request.reply.send(result);
                            }
                        }
                        Err(error) => {
                            panel.set(true);
                            message.error(error.to_string());
                            #[cfg(target_os = "linux")]
                            if let Some(request) = activation {
                                let _ = request.reply.send(Err(error.to_string()));
                            }
                        }
                    }
                }
            }
        }
        rect().maybe_child(job.read().as_ref().map(|current| {
            rect()
                .horizontal()
                .spacing(t::SPACE_XS)
                .child(
                    label()
                        .text(if current.cancelled {
                            "Finishing cancelled load…"
                        } else {
                            "Opening project…"
                        })
                        .font_size(t::small()),
                )
                .child(
                    Button::new()
                        .flat()
                        .enabled(!current.cancelled)
                        .on_press(move |_| {
                            if let Some(current) = job.write().as_mut() {
                                current.cancelled = true;
                            }
                        })
                        .child(crate::messages::text(
                            crate::messages::MsgId::WriterGuiCancelOpening,
                        )),
                )
        }))
    }
}
