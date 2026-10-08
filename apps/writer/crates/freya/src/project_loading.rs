//! One project load at a time; cancelled or superseded work never replaces edits.
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
    project::ProjectFiles,
};
use freya::prelude::*;
use std::path::PathBuf;
mod completion;
mod worker;

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
    load: worker::Load,
    source: Option<std::sync::Arc<str>>,
    #[cfg(target_os = "linux")]
    activation: Option<crate::activation::IncomingRoute>,
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
    #[cfg(target_os = "linux")]
    let forwarded = activation.as_ref().map(|request| request.cancelled.clone());
    #[cfg(not(target_os = "linux"))]
    let forwarded = None;
    match worker::Load::open(path, forwarded) {
        Ok(load) => {
            job.set(Some(LoadJob {
                load,
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
        let files = self.files;
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
            completion::poll(self);
        }
        rect().maybe_child(job.read().as_ref().map(|current| {
            rect()
                .horizontal()
                .spacing(t::SPACE_XS)
                .child(
                    label()
                        .text(if current.load.cancelled() {
                            "Finishing cancelled load…"
                        } else {
                            "Opening project…"
                        })
                        .font_size(t::small()),
                )
                .child(
                    Button::new()
                        .flat()
                        .enabled(!current.load.cancelled())
                        .on_press(move |_| {
                            if let Some(current) = job.write().as_mut() {
                                current.load.cancel();
                            }
                        })
                        .child(crate::messages::text(
                            crate::messages::MsgId::WriterGuiCancelOpening,
                        )),
                )
        }))
    }
}
