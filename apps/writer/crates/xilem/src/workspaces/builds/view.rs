use super::*;
use crate::WriterLayout;
use crate::attention::{Pane, PaneEntry, Region};
use crate::presentation::space;
use crate::presentation::{group::named, label, paragraph, print, radio_button, text_button};
use xilem::{
    WidgetView,
    core::one_of::Either,
    masonry::{layout::AsUnit, properties::LineBreaking},
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
        crate::view::tool_head(
            writer,
            "Build report",
            build_actions(
                writer,
                busy,
                builds.is_none_or(|builds| builds.targets.is_empty()),
            ),
        )
        .prop(PaneEntry(true)),
        inputs(writer).map(|inputs| paragraph(scale, inputs, width)),
        feedback(writer),
        writer.workspaces.build_feedback.is_empty().then(|| {
            paragraph(
                scale,
                if builds.is_none_or(|builds| builds.targets.is_empty()) {
                    "This project has no outputs set up yet, so there is nothing to build. Add a \
                     [[scenes]] entry with an id, an asset path and a start block to \
                     recite.project.toml, then reopen this page."
                } else {
                    "No builds yet. Build saved files writes your outputs; the result appears \
                     above the status line."
                },
                width,
            )
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
    .prop(Region(Some(Pane::Build)))
    // The head spans the page, so its close sits at the sheet's edge; the report keeps its measure.
    .width(writer.page_width().px())
}
fn column_width(writer: &Writer) -> f64 {
    writer.page_width().min(800. * f64::from(writer.ui_scale()))
}
/// What a build reads and writes, in the writer's words: the scene files and the outputs.
pub(super) fn inputs(writer: &Writer) -> Option<String> {
    let project = writer.session.project()?;
    let sources = project.documents().len();
    let targets = project.files().builds.targets.len();
    Some(format!(
        "{sources} {} · {}",
        if sources == 1 {
            "source file"
        } else {
            "source files"
        },
        match targets {
            0 => "no outputs set up".to_owned(),
            1 => "1 output".to_owned(),
            _ => format!("{targets} outputs"),
        },
    ))
}
/// Whether a build cannot start now: the project is busy, has no outputs, or is closing.
pub(super) fn build_blocked(writer: &Writer, busy: bool, empty: bool) -> bool {
    writer.declaration_action_blocked()
        || busy
        || empty
        || writer.phase() != Phase::Writing
        || writer.navigation.busy()
        || writer.work.busy()
}
pub(super) fn build_actions(
    writer: &Writer,
    busy: bool,
    empty: bool,
) -> impl WidgetView<Writer> + use<> {
    let blocked = build_blocked(writer, busy, empty);
    // The sheet's own action is its one primary; Save stays tonal in the toolbar.
    flex_row((
        text_button(
            writer.ui_scale(),
            "Save all and build",
            Writer::save_and_build,
        )
        .disabled(blocked),
        print::primary_action(
            writer.ui_scale(),
            "Build saved files",
            &writer.palette,
            blocked,
            Writer::build_saved,
        ),
    ))
    .gap(space::SIBLINGS.px())
}
/// The outputs to build: one target as words, several as a radio group with All outputs first.
pub(super) fn target_options(writer: &Writer, width: f64) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let builds = writer
        .session
        .project()
        .map(|project| &project.files().builds);
    let busy = builds.is_some_and(|builds| builds.busy());
    let choices = builds
        .into_iter()
        .flat_map(|builds| {
            builds.targets.iter().map(move |(asset, scenes)| {
                target_choice(
                    writer,
                    width,
                    asset,
                    scenes,
                    builds.selected.as_ref() == Some(asset),
                    busy,
                )
            })
        })
        .collect::<Vec<_>>();
    if choices.len() <= 1 {
        // One output (or none) is not a choice: say what a build writes.
        let words = builds
            .and_then(|builds| builds.targets.first())
            .map_or_else(
                || "Builds every output".to_owned(),
                |(asset, scenes)| format!("Builds {} · {asset}", scenes.join(", ")),
            );
        return Either::A(
            label(scale, words)
                .prop(LineBreaking::WordWrap)
                .width(width.px()),
        );
    }
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
    Either::B(named(
        "Build target",
        radio_group(
            flex_col((all, choices))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(space::TEXT.px()),
        ),
    ))
}

fn target_choice(
    writer: &Writer,
    width: f64,
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
            print::id(scale, asset.to_owned())
                .color(writer.palette.muted)
                .prop(LineBreaking::WordWrap)
                .width((width - 44. * f64::from(scale)).px()),
        )
        .padding(Padding::from_vh(0.px(), (22. * f64::from(scale)).px())),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::UNIT.px())
}

/// The latest build's report, every part of it, on inset paper.
fn feedback(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let parts = &writer.workspaces.build_feedback;
    (!parts.is_empty()).then(|| {
        let scale = writer.ui_scale();
        let width = column_width(writer) - 2. * space::KINDS;
        sized_box(
            flex_col((
                print::section_heading(scale, "Latest build"),
                parts
                    .iter()
                    .map(|part| paragraph(scale, part.clone(), width))
                    .collect::<Vec<_>>(),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px()),
        )
        .padding(space::KINDS.px())
        .background_color(writer.palette.inset)
    })
}
