//! A build's result as a bar seated on the status line that also reports it, across the window:
//! the outcome on one line, Build options as a quiet disclosure (or a failure's way to its report
//! page), the build as the bar's one Primary, and a dismiss icon; in a narrow window the actions
//! take a second row. The options open inside the bar: what a build reads, the outputs to build
//! and Save all and build. It does not take focus; the writer keeps writing.
use super::*;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::sheet_text::one_line;
use crate::presentation::tokens::size;
use crate::presentation::{Icon, Type, disclosure, group::named, pane, print, space};
use recite_writer_app::COMPACT_WINDOW;
use xilem::{
    WidgetView,
    core::one_of::Either,
    masonry::{layout::AsUnit, parley::style::FontWeight, properties::LineBreaking},
    style::{Padding, Style},
    view::{CrossAxisAlignment, FlexExt, FlexSpacer, button, flex_col, flex_row, sized_box},
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
    // The message keeps one line, ending in an ellipsis where it would not fit; the status
    // sentence and the bar's description carry it whole.
    let message = result.detail.as_ref().map_or_else(
        || result.headline.clone(),
        |detail| format!("{} · {detail}", result.headline),
    );
    let outcome = flex_row((
        one_line(result.headline.clone(), Type::Control, scale, palette.ink)
            .weight(FontWeight::SEMI_BOLD),
        result
            .detail
            .clone()
            .map(|detail| one_line(detail, Type::Meta, scale, palette.muted).flex(1.)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(space::SIBLINGS.px())
    .flex(1.);
    // Where the window is narrow for the UI scale, the actions take a second row.
    let narrow = writer.viewport.width < COMPACT_WINDOW * f64::from(scale);
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
    let build = print::primary_action(
        scale,
        "Build saved files",
        palette,
        super::view::build_blocked(writer, busy, empty),
        Writer::build_saved,
    );
    let dismiss = print::icon_action(
        scale,
        "Dismiss build result",
        Icon::Close,
        palette,
        false,
        Writer::dismiss_build_result,
    );
    let rows = if narrow {
        Either::A(flex_col((
            flex_row((outcome, dismiss))
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(space::SIBLINGS.px()),
            flex_row((FlexSpacer::Flex(1.), way, build))
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(space::SIBLINGS.px()),
        )))
    } else {
        Either::B(
            flex_row((outcome, way, build, dismiss))
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(space::SIBLINGS.px()),
        )
    };
    Some(
        named(
            "Build result",
            sized_box(
                flex_col((rows, open.then(|| capped(writer, narrow, busy, empty))))
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .gap(space::SIBLINGS.px()),
            )
            .padding(Padding::from_vh(space::SIBLINGS.px(), space::KINDS.px()))
            .background_color(palette.floating)
            .border_color(palette.rule)
            .border_width(1.px()),
        )
        .description(message),
    )
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

/// The most of the window the open bar takes; its options scroll inside the rest.
const BAR_SHARE: f64 = 0.35;

/// The options under the bar's rows, scrolling in a pane where the bar would take more than
/// [`BAR_SHARE`] of the window, so the work above keeps most of it.
fn capped(
    writer: &Writer,
    narrow: bool,
    busy: bool,
    empty: bool,
) -> impl WidgetView<Writer> + use<> {
    let scale = f64::from(writer.ui_scale());
    let rows = if narrow {
        2. * size::ROW * scale + space::SIBLINGS
    } else {
        size::ROW * scale
    };
    // The bar's padding and the gap above the options.
    let room =
        (BAR_SHARE * writer.viewport.height - rows - 3. * space::SIBLINGS).max(size::ROW * scale);
    let targets = writer
        .session
        .project()
        .map_or(0, |project| project.files().builds.targets.len());
    // A Meta line, the outputs (one sentence, or a choice per output and All), and the action.
    let outputs = if targets <= 1 {
        f64::from(Type::Body.metrics().1) * scale
    } else {
        (targets + 1) as f64 * size::ROW * scale
    };
    let natural = f64::from(Type::Meta.metrics().1) * scale
        + outputs
        + size::ROW * scale
        + 2. * space::SIBLINGS;
    if natural <= room {
        Either::A(options(writer, busy, empty))
    } else {
        Either::B(
            sized_box(
                pane("Build outputs", options(writer, busy, empty)).constrain_horizontal(true),
            )
            .height(room.px()),
        )
    }
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
