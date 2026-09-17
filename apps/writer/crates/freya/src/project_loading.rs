//! One project load at a time; cancelled or superseded work never replaces edits.
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
    project::{FileError, ProjectFiles},
};
use freya::prelude::*;
use std::{path::PathBuf, sync::mpsc, thread::JoinHandle};

pub(crate) struct LoadJob {
    result: mpsc::Receiver<Result<(ProjectFiles, recite_writer_model::Workbench), FileError>>,
    worker: Option<JoinHandle<()>>,
    cancelled: bool,
    source: Option<std::sync::Arc<str>>,
}
impl Drop for LoadJob {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
pub(crate) fn start(
    mut job: State<Option<LoadJob>>,
    path: PathBuf,
    mut message: State<String>,
    buffers: crate::buffers::Buffers,
) {
    if job.peek().is_some() {
        message.set("A project is still opening. Cancel it or wait for it to finish.".into());
        return;
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
        Ok(worker) => job.set(Some(LoadJob {
            result,
            worker: Some(worker),
            cancelled: false,
            source: buffers
                .model
                .peek()
                .as_ref()
                .ok()
                .map(|m| m.document().source_snapshot()),
        })),
        Err(error) => message.set(format!("Could not start opening the project: {error}")),
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
        let mut job = self.job;
        let mut files = self.files;
        let mut panel = self.panel;
        let mut message = self.writer.message;
        if tick.elapsed() {
            tick.reset();
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
                job.set(None);
                if cancelled {
                    message.set("Project opening cancelled.".into());
                } else if source != current || !self.writer.buffers.can_leave(files.peek().as_ref())
                {
                    message.set("The current document changed while loading. Save it before opening another project.".into());
                } else {
                    match loaded {
                        Ok((project, workbench)) => {
                            let recovered = project.has_recovery();
                            self.writer.buffers.install(workbench, self.writer.dark);
                            files.set(Some(project));
                            self.writer.scene_opened();
                            panel.set(false);
                            message.set(
                                if recovered {
                                    "Recovered your previous session."
                                } else {
                                    "Project opened."
                                }
                                .into(),
                            );
                        }
                        Err(error) => {
                            panel.set(true);
                            message.set(error.to_string());
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
                        .font_size(t::TEXT_SMALL),
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
                        .child("Cancel opening"),
                )
        }))
    }
}
