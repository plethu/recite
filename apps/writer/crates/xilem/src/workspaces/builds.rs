use super::*;
use crate::attention::{Pane, Region};
use crate::presentation::{label, paragraph, text_button};
use recite_writer_session::builds::job;
use xilem::{
    WidgetView,
    masonry::layout::AsUnit,
    style::Style,
    view::{CrossAxisAlignment, flex_col, flex_row},
};

impl Writer {
    pub(crate) fn build_saved(&mut self) {
        let selected = self
            .session
            .project()
            .and_then(|project| project.files().builds.selected.clone());
        self.build_selected(selected);
    }
    pub(crate) fn build_selected(&mut self, asset: Option<String>) {
        if self.phase != Phase::Writing || self.navigation.busy() || self.work.busy() {
            return;
        }
        let Some(project) = self.session.project_mut() else {
            return;
        };
        let (files, _) = project.parts_mut();
        if files.builds.busy() {
            return;
        }
        match job::Job::start(files.root().to_owned(), asset) {
            Ok(job) => {
                self.workspaces.build_feedback.clear();
                self.workspaces.feedback_page = 0;
                files.builds.job = Some(job);
                files.builds.result = None;
                self.status = "Building saved project inputs… You can keep writing.".into();
            }
            Err(error) => self.status = error,
        }
    }
    pub(crate) fn cancel_build(&mut self) {
        if let Some(project) = self.session.project() {
            project.files().builds.cancel();
            self.status = "Cancelling build; waiting for its terminal result.".into();
        }
    }
    pub(crate) fn poll_project_jobs(&mut self) {
        self.poll_schema_generation();
        let Some(project) = self.session.project_mut() else {
            return;
        };
        let (files, _) = project.parts_mut();
        if let Some(result) = files.builds.job.as_ref().and_then(job::Job::poll) {
            let message = result.message();
            self.status = message
                .lines()
                .next()
                .unwrap_or("Build finished")
                .chars()
                .take(240)
                .collect();
            self.workspaces.build_feedback = super::text::pages(&message);
            self.workspaces.feedback_page = 0;
            files.builds.job = None;
            files.builds.result = Some(result);
        }
    }
}
pub(super) fn panel(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let builds = writer
        .session
        .project()
        .map(|project| &project.files().builds);
    let busy = builds.is_some_and(|builds| builds.busy());
    flex_col((
        label(scale, "Build scenes").text_size(28. * scale),
        label(scale,"Build target").text_size(18.*scale),
        target_options(writer),
        pagination(writer),
        paragraph(scale, "Build uses saved project inputs. Save all before building to include your current edits.", 640.),
        build_actions(writer, busy, builds.is_none_or(|builds| builds.targets.is_empty())),
        feedback(writer),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
    .prop(Region(Some(Pane::Build)))
}
fn build_actions(writer: &Writer, busy: bool, empty: bool) -> impl WidgetView<Writer> + use<> {
    let blocked = busy
        || empty
        || writer.phase != Phase::Writing
        || writer.navigation.busy()
        || writer.work.busy();
    flex_row((
        text_button(writer.ui_scale(), "Build saved", Writer::build_saved).disabled(blocked),
        text_button(
            writer.ui_scale(),
            "Save all and build",
            Writer::save_and_build,
        )
        .disabled(blocked),
    ))
}
fn pagination(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let last = writer.session.project().is_none_or(|project| {
        (writer.workspaces.build_page + 1) * 5 >= project.files().builds.targets.len()
    });
    flex_row((
        text_button(
            writer.ui_scale(),
            "Previous outputs",
            |writer: &mut Writer| {
                writer.workspaces.build_page = writer.workspaces.build_page.saturating_sub(1);
            },
        )
        .disabled(writer.workspaces.build_page == 0),
        text_button(writer.ui_scale(), "Next outputs", |writer: &mut Writer| {
            writer.workspaces.build_page += 1
        })
        .disabled(last),
    ))
}

fn target_options(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let builds = writer
        .session
        .project()
        .map(|project| &project.files().builds);
    let busy = builds.is_some_and(|builds| builds.busy());
    flex_col((
        text_button(
            writer.ui_scale(),
            if builds.is_none_or(|builds| builds.selected.is_none()) {
                "Selected · All outputs"
            } else {
                "All outputs"
            },
            |writer: &mut Writer| {
                if let Some(project) = writer.session.project_mut() {
                    project.parts_mut().0.builds.selected = None;
                }
            },
        )
        .disabled(busy),
        builds
            .into_iter()
            .flat_map(|builds| {
                builds
                    .targets
                    .iter()
                    .skip(writer.workspaces.build_page * 5)
                    .take(5)
                    .map(move |(asset, scenes)| {
                        let selected = builds.selected.as_ref() == Some(asset);
                        let asset = asset.clone();
                        text_button(
                            writer.ui_scale(),
                            format!(
                                "{} · {} · {asset}",
                                if selected { "Selected" } else { "Output" },
                                scenes.join(", ")
                            ),
                            move |writer: &mut Writer| {
                                if let Some(project) = writer.session.project_mut() {
                                    project.parts_mut().0.builds.selected = Some(asset.clone());
                                }
                            },
                        )
                        .disabled(busy)
                    })
            })
            .collect::<Vec<_>>(),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
}

fn feedback(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let report = writer
        .workspaces
        .build_feedback
        .get(writer.workspaces.feedback_page)?;
    Some(
        flex_col((
            paragraph(writer.ui_scale(), report.clone(), 640.),
            flex_row((
                text_button(
                    writer.ui_scale(),
                    "Previous build report",
                    |writer: &mut Writer| {
                        writer.workspaces.feedback_page =
                            writer.workspaces.feedback_page.saturating_sub(1);
                    },
                )
                .disabled(writer.workspaces.feedback_page == 0),
                text_button(
                    writer.ui_scale(),
                    "More build report",
                    |writer: &mut Writer| writer.workspaces.feedback_page += 1,
                )
                .disabled(
                    writer.workspaces.feedback_page + 1 >= writer.workspaces.build_feedback.len(),
                ),
            )),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start),
    )
}
