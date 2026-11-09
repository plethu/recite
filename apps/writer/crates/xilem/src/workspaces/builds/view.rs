use super::*;
use crate::attention::{Pane, PaneEntry, Region};
use crate::presentation::{group::named, label, paragraph, print, radio_button, text_button};
use xilem::{
    WidgetView,
    masonry::{kurbo::Axis, layout::AsUnit, properties::LineBreaking},
    style::{Padding, Style},
    view::{CrossAxisAlignment, flex_col, flex_row, radio_group, sized_box},
};

pub(in crate::workspaces) fn panel(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let builds = writer
        .session
        .project()
        .map(|project| &project.files().builds);
    let busy = builds.is_some_and(|builds| builds.busy());
    let width = column_width(writer);
    flex_col((
        flex_row((
            label(scale, "Build scenes").text_size(24. * scale),
            build_actions(writer, busy, builds.is_none_or(|builds| builds.targets.is_empty())),
        )).direction(if width >= 580. * f64::from(scale) { Axis::Horizontal } else { Axis::Vertical })
            .cross_axis_alignment(CrossAxisAlignment::Center).gap(24.px()).prop(PaneEntry(true)),
        writer.session.project().map(|project| {
            let sources = project.documents().len();
            let targets = builds.map_or(0, |builds| builds.targets.len());
            paragraph(scale, format!(
                "recite.project.toml · {sources} {} · {targets} {}",
                if sources == 1 { "source file" } else { "source files" },
                if targets == 1 { "output target" } else { "output targets" },
            ), width)
        }),
        paragraph(scale, "Every build validates the whole saved project. The selected target limits emitted assets. Save all and build includes your current edits.", width),
        feedback(writer),
        label(scale,"Build target").text_size(18.*scale),
        target_options(writer),
        pagination(writer),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
    .prop(Region(Some(Pane::Build)))
    .width(width.px())
}
fn column_width(writer: &Writer) -> f64 {
    writer
        .content_width()
        .min(800. * f64::from(writer.ui_scale()))
}
fn build_actions(writer: &Writer, busy: bool, empty: bool) -> impl WidgetView<Writer> + use<> {
    let blocked = writer.declaration_action_blocked()
        || busy
        || empty
        || writer.phase() != Phase::Writing
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
fn pagination(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let count = writer.session.project()?.files().builds.targets.len();
    if count <= 5 {
        return None;
    }
    let last = writer.session.project().is_none_or(|project| {
        (writer.workspaces.build_page + 1) * 5 >= project.files().builds.targets.len()
    });
    Some(flex_row((
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
    )))
}

fn target_options(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let builds = writer
        .session
        .project()
        .map(|project| &project.files().builds);
    let busy = builds.is_some_and(|builds| builds.busy());
    let choices = builds
        .into_iter()
        .flat_map(|builds| {
            builds
                .targets
                .iter()
                .skip(writer.workspaces.build_page * 5)
                .take(5)
                .map(move |(asset, scenes)| {
                    target_choice(
                        writer,
                        asset,
                        scenes,
                        builds.selected.as_ref() == Some(asset),
                        busy,
                    )
                })
        })
        .collect::<Vec<_>>();
    let all = radio_button(
        scale,
        "All outputs",
        builds.is_none_or(|builds| builds.selected.is_none()),
        |writer: &mut Writer| {
            if let Some(project) = writer.session.project_mut() {
                project.parts_mut().0.builds.selected = None;
            }
        },
    )
    .disabled(busy);
    named(
        "Build target",
        radio_group(
            flex_col((all, choices))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(12.px()),
        ),
    )
}

fn target_choice(
    writer: &Writer,
    asset: &str,
    scenes: &[String],
    selected: bool,
    busy: bool,
) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let path = asset.to_owned();
    let choice = radio_button(
        scale,
        scenes.join(", "),
        selected,
        move |writer: &mut Writer| {
            if let Some(project) = writer.session.project_mut() {
                project.parts_mut().0.builds.selected = Some(path.clone());
            }
        },
    )
    .disabled(busy);
    flex_col((
        choice,
        sized_box(
            print::meta(scale, asset.to_owned())
                .color(writer.palette.muted)
                .prop(LineBreaking::WordWrap)
                .width((column_width(writer) - 44. * f64::from(scale)).px()),
        )
        .padding(Padding::from_vh(0.px(), (22. * f64::from(scale)).px())),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(4.px())
}

fn feedback(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let report = writer
        .workspaces
        .build_feedback
        .get(writer.workspaces.feedback_page)?;
    Some(
        sized_box(
            flex_col((
                label(writer.ui_scale(), "Latest build").text_size(18. * writer.ui_scale()),
                paragraph(
                    writer.ui_scale(),
                    report.clone(),
                    column_width(writer) - 32.,
                ),
                (writer.workspaces.build_feedback.len() > 1).then(|| {
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
                            writer.workspaces.feedback_page + 1
                                >= writer.workspaces.build_feedback.len(),
                        ),
                    ))
                }),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px()),
        )
        .padding(16.px())
        .background_color(writer.palette.inset),
    )
}
