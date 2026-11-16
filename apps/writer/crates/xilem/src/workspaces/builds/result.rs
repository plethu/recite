//! A build's result on a slip laid over the foot of the window, just above the status line that
//! also reports it: the summary, the way to the build page (a failure's report, or the outputs to
//! build) and a dismiss icon. It does not take focus; the writer keeps writing.
use super::*;
use crate::presentation::slip::{OFFSET, slip};
use crate::presentation::tokens::size;
use crate::presentation::{Icon, Type, group::named, print, space, text_button};
use xilem::{
    WidgetView,
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
                    text_button(
                        scale,
                        if result.succeeded {
                            "Build options"
                        } else {
                            "Show build report"
                        },
                        Writer::open_build_page,
                    ),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(space::SIBLINGS.px()),
            )
            .padding(Padding::from_vh(space::TEXT.px(), space::KINDS.px()))
            .width(width.px()),
        ),
    ))
}
