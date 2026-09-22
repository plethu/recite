//! Manifest-selected builds run independently of screen navigation.
mod job;
use crate::{
    design::{Button, SubmitAction, tokens as t},
    editing::{Pane, Writer},
    messages::{MsgId, text},
};
use freya::prelude::*;
#[derive(Default)]
pub(crate) struct Builds {
    targets: Vec<(String, Vec<String>)>,
    selected: Option<String>,
    job: Option<job::Job>,
    result: Option<Result<String, String>>,
}
impl Builds {
    pub(crate) fn cancel(&self) {
        if let Some(job) = &self.job {
            job.control.cancel();
        }
    }
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
}
pub(crate) fn open(mut writer: Writer) {
    if let Err(error) = try_open(writer) {
        writer.message.error(error);
    }
}
pub(crate) fn try_open(mut writer: Writer) -> Result<(), String> {
    let mut files = writer.files.write();
    let project = files.as_mut().ok_or("Open a project first.")?;
    if !project.builds.busy() {
        match recite_config::discover_project(project.root()) {
            Ok(report) => {
                let mut targets = std::collections::BTreeMap::<String, Vec<String>>::new();
                for scene in &report.manifest().source().manifest().scenes {
                    targets
                        .entry(scene.asset.clone())
                        .or_default()
                        .push(scene.id.clone());
                }
                project.builds.targets = targets.into_iter().collect();
                if project.builds.selected.as_ref().is_some_and(|selected| {
                    !project
                        .builds
                        .targets
                        .iter()
                        .any(|(asset, _)| asset == selected)
                }) {
                    project.builds.selected = None;
                }
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    writer.pane.set(Pane::Build);
    Ok(())
}
fn start(mut writer: Writer, save: bool) {
    if save && let Err(e) = writer.buffers.save_all(writer.files) {
        writer.message.error(e);
        return;
    }
    let mut files = writer.files.write();
    let Some(project) = files.as_mut() else {
        return;
    };
    if project.builds.busy() {
        return;
    }
    match job::Job::start(project.root().to_owned(), project.builds.selected.clone()) {
        Ok(job) => {
            project.builds.job = Some(job);
            project.builds.result = None;
        }
        Err(e) => writer.message.error(e),
    }
}
#[derive(Clone, Copy)]
pub(crate) struct BuildScreen {
    pub writer: Writer,
}
impl PartialEq for BuildScreen {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}
impl Component for BuildScreen {
    fn render(&self) -> impl IntoElement {
        let mut writer = self.writer;
        let id = use_a11y();
        let files = writer.files.read();
        let Some(project) = files.as_ref() else {
            return rect();
        };
        let builds = &project.builds;
        let busy = builds.busy();
        let submit = SubmitAction {
            id,
            caption: text(MsgId::WriterBuildSaved),
            enabled: !busy && !builds.targets.is_empty(),
            action: EventHandler::new(move |()| start(writer, false)),
        };
        let keyboard = submit.clone();
        let mut body = rect()
            .width(Size::fill())
            .spacing(t::SPACE_LG)
            .padding(t::SPACE_XL)
            .on_key_down(move |e: Event<KeyboardEventData>| {
                if crate::design::keyboard::submit_key(&e) {
                    e.prevent_default();
                    e.stop_propagation();
                    keyboard.run();
                }
            })
            .child(
                rect()
                    .horizontal()
                    .spacing(t::SPACE_LG)
                    .child(
                        label()
                            .text(text(MsgId::WriterBuildScenes))
                            .font_size(t::title()),
                    )
                    .child(
                        Button::new()
                            .flat()
                            .on_press(move |_| writer.pane.set(Pane::Map))
                            .child(text(MsgId::WriterReturnWriting)),
                    ),
            )
            .child(
                label()
                    .text(
                        project
                            .root()
                            .join("recite.project.toml")
                            .display()
                            .to_string(),
                    )
                    .color(t::colors().muted),
            );
        let mut options = rect().spacing(t::SPACE_SM).child(
            Button::new()
                .radio(builds.selected.is_none())
                .enabled(!busy)
                .on_press(move |_| {
                    if let Some(project) = writer.files.write().as_mut() {
                        project.builds.selected = None;
                    }
                })
                .child(text(MsgId::WriterBuildAll)),
        );
        for (asset, scenes) in &builds.targets {
            let target = asset.clone();
            options = options.child(
                Button::new()
                    .radio(builds.selected.as_ref() == Some(asset))
                    .enabled(!busy)
                    .on_press(move |_| {
                        if let Some(project) = writer.files.write().as_mut() {
                            project.builds.selected = Some(target.clone());
                        }
                    })
                    .child(format!("{} · {asset}", scenes.join(", "))),
            );
        }
        body = body
            .child(options)
            .child(
                paragraph()
                    .width(Size::fill())
                    .span(Span::new(text(MsgId::WriterBuildScope))),
            )
            .child(label().text(format!(
                    "{}: {}",
                    text(MsgId::WriterBuildInputs),
                    project
                        .paths
                        .iter()
                        .filter_map(|p| p.strip_prefix(project.root()).ok())
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
        if busy {
            body = body.child(
                rect()
                    .horizontal()
                    .spacing(t::SPACE_MD)
                    .child(
                        label().text(text(
                            if builds
                                .job
                                .as_ref()
                                .is_some_and(|j| j.control.cancellation().is_some())
                            {
                                MsgId::WriterBuildCancelling
                            } else {
                                MsgId::WriterBuilding
                            },
                        )),
                    )
                    .child(
                        Button::new()
                            .enabled(
                                builds
                                    .job
                                    .as_ref()
                                    .is_some_and(|j| j.control.cancellation().is_none()),
                            )
                            .on_press(move |_| {
                                if let Some(job) = writer
                                    .files
                                    .peek()
                                    .as_ref()
                                    .and_then(|p| p.builds.job.as_ref())
                                {
                                    job.control.cancel();
                                }
                            })
                            .child(text(MsgId::WriterCancel)),
                    ),
            );
        } else {
            body = body.child(
                rect()
                    .horizontal()
                    .spacing(t::SPACE_MD)
                    .child(submit.button())
                    .child(
                        Button::new()
                            .enabled(!builds.targets.is_empty())
                            .on_press(move |_| start(writer, true))
                            .child(text(MsgId::WriterSaveBuild)),
                    ),
            );
        }
        if let Some(result) = &builds.result {
            body = body.child(
                paragraph()
                    .width(Size::fill())
                    .span(Span::new(match result {
                        Ok(s) | Err(s) => s.clone(),
                    })),
            );
        }
        rect().width(Size::fill()).height(Size::fill()).child(
            ScrollView::new()
                .width(Size::fill())
                .height(Size::fill())
                .child(body),
        )
    }
}
#[derive(Clone, Copy)]
pub(crate) struct BuildPoll {
    pub writer: Writer,
}
impl PartialEq for BuildPoll {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}
impl Component for BuildPoll {
    fn render(&self) -> impl IntoElement {
        let mut writer = self.writer;
        let mut tick = freya::sdk::use_timeout(|| std::time::Duration::from_millis(100));
        if tick.elapsed() {
            tick.reset();
            let result = writer
                .files
                .peek()
                .as_ref()
                .and_then(|f| f.builds.job.as_ref())
                .and_then(job::Job::poll);
            if let Some(result) = result {
                writer.message.report(
                    result.clone().map(|_| ()),
                    result.as_ref().cloned().unwrap_or_default(),
                );
                if let Some(project) = writer.files.write().as_mut() {
                    project.builds.job = None;
                    project.builds.result = Some(result);
                }
            }
        }
        rect()
    }
}
