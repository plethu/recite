//! A build's result as a bar seated on the status line that also reports it, across the window:
//! the outcome, Build options as a quiet disclosure (or a failure's way to its report page), the
//! build as the bar's one Primary, and a dismiss icon. The options open inside the bar: what a
//! build reads, the outputs to build and Save all and build. It does not take focus; the writer
//! keeps writing.
use super::*;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::tokens::size;
use crate::presentation::{Icon, Type, disclosure, group::named, print, space};
use xilem::{
    WidgetView,
    core::one_of::Either,
    masonry::{layout::AsUnit, parley::style::FontWeight, properties::LineBreaking},
    style::{Padding, Style},
    view::{CrossAxisAlignment, FlexExt, button, flex_col, flex_row, sized_box},
};

pub(crate) fn result(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    // Behind the close decision only the decision is live.
    let result = writer
        .build_result()
        .filter(|_| writer.phase() == Phase::Writing)?;
    let scale = writer.ui_scale();
    let palette = &writer.palette;
    let open = result.succeeded && writer.workspaces.build_options;
    let (busy, empty) = state(writer);
    let outcome = flex_row((
        Type::Control
            .label(scale, result.headline)
            .weight(FontWeight::SEMI_BOLD)
            .prop(LineBreaking::Clip),
        result.detail.map(|detail| {
            print::meta(scale, detail)
                .color(palette.muted)
                .prop(LineBreaking::Clip)
                .flex(1.)
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(space::SIBLINGS.px())
    .flex(1.);
    let way = if result.succeeded {
        Either::A(styled(
            ControlStyle::Action,
            button(
                disclosure(
                    scale,
                    open,
                    palette.ink,
                    Type::Control.label(scale, "Build options"),
                ),
                Writer::toggle_build_options,
            )
            .expanded(open),
        ))
    } else {
        Either::B(print::action(
            scale,
            "Show build report",
            false,
            Writer::open_build_page,
        ))
    };
    Some(named(
        "Build result",
        sized_box(
            flex_col((
                flex_row((
                    outcome,
                    way,
                    print::primary_action(
                        scale,
                        "Build saved",
                        palette,
                        super::view::build_blocked(writer, busy, empty),
                        Writer::build_saved,
                    ),
                    print::icon_action(
                        scale,
                        "Dismiss build result",
                        Icon::Close,
                        palette,
                        false,
                        Writer::dismiss_build_result,
                    ),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(space::SIBLINGS.px()),
                open.then(|| options(writer, busy, empty)),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(space::SIBLINGS.px()),
        )
        .padding(Padding::from_vh(space::SIBLINGS.px(), space::KINDS.px()))
        .background_color(palette.floating)
        .border_color(palette.rule)
        .border_width(1.px()),
    ))
}

/// Whether the project's builds are busy, and whether it has no outputs.
fn state(writer: &Writer) -> (bool, bool) {
    let builds = writer
        .session
        .project()
        .map(|project| &project.files().builds);
    (
        builds.is_some_and(|builds| builds.busy()),
        builds.is_none_or(|builds| builds.targets.is_empty()),
    )
}

/// The build options inside the bar: what a build reads, the outputs to build, and Save all and
/// build beside the bar's own Primary.
fn options(writer: &Writer, busy: bool, empty: bool) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let width =
        (size::SLIP_WIDTH * f64::from(scale)).min(writer.viewport.width - 2. * space::KINDS);
    flex_col((
        super::view::inputs(writer).map(|inputs| {
            print::meta(scale, inputs)
                .color(writer.palette.muted)
                .prop(LineBreaking::WordWrap)
        }),
        super::view::target_options(writer, width),
        print::action(
            scale,
            "Save all and build",
            super::view::build_blocked(writer, busy, empty),
            Writer::save_and_build,
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
}
