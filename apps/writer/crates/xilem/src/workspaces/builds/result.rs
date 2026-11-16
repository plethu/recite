//! A build's result on a slip seated on the status line that also reports it: the summary, a
//! dismiss icon, and either a failure's way to its report page or the build options, which open on
//! the slip itself: the outputs to build and the build actions. It does not take focus; the writer
//! keeps writing.
use super::*;
use crate::presentation::slip::{OFFSET, slip};
use crate::presentation::tokens::size;
use crate::presentation::{Icon, Type, group::named, print, space, text_button};
use xilem::{
    WidgetView,
    core::one_of::Either,
    masonry::{layout::AsUnit, parley::style::FontWeight, properties::LineBreaking},
    style::{Padding, Style},
    view::{CrossAxisAlignment, FlexExt, flex_col, flex_row, sized_box},
};

pub(crate) fn result(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    // Behind the close decision only the decision is live.
    let result = writer
        .build_result()
        .filter(|_| writer.phase() == Phase::Writing)?;
    let scale = writer.ui_scale();
    let width = (size::SLIP_WIDTH * f64::from(scale))
        .min(writer.viewport.width - 2. * space::KINDS - OFFSET);
    Some(named(
        "Build result",
        slip(
            &writer.palette,
            sized_box(
                flex_col((
                    flex_row((
                        Type::Control
                            .label(scale, result.headline)
                            .weight(FontWeight::SEMI_BOLD)
                            .prop(LineBreaking::WordWrap)
                            .flex(1.),
                        print::icon_action(
                            scale,
                            "Dismiss build result",
                            Icon::Close,
                            &writer.palette,
                            false,
                            Writer::dismiss_build_result,
                        ),
                    ))
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .gap(space::SIBLINGS.px()),
                    result.detail.map(|detail| {
                        print::meta(scale, detail)
                            .color(writer.palette.muted)
                            .prop(LineBreaking::WordWrap)
                    }),
                    if result.succeeded {
                        Either::A(
                            text_button(scale, "Build options", Writer::toggle_build_options)
                                .expanded(writer.workspaces.build_options),
                        )
                    } else {
                        Either::B(text_button(
                            scale,
                            "Show build report",
                            Writer::open_build_page,
                        ))
                    },
                    (result.succeeded && writer.workspaces.build_options)
                        .then(|| options(writer, width - 2. * space::KINDS)),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(space::SIBLINGS.px()),
            )
            .padding(Padding::from_vh(space::TEXT.px(), space::KINDS.px()))
            .width(width.px()),
        ),
    ))
}

/// The build options on the slip: what a build reads, the outputs to build, and the actions.
fn options(writer: &Writer, width: f64) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let builds = writer
        .session
        .project()
        .map(|project| &project.files().builds);
    let busy = builds.is_some_and(|builds| builds.busy());
    let empty = builds.is_none_or(|builds| builds.targets.is_empty());
    flex_col((
        super::view::inputs(writer).map(|inputs| {
            print::meta(scale, inputs)
                .color(writer.palette.muted)
                .prop(LineBreaking::WordWrap)
        }),
        super::view::target_options(writer, width),
        super::view::build_actions(writer, busy, empty),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
}
