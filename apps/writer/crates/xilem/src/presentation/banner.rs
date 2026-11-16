//! Banners: a notice across the top of the sheet it concerns, on inset paper with a 2px ink bar
//! down its leading edge. The message keeps one line and the actions follow it, or take a second
//! line inside the banner when the row is too narrow for both; a banner never becomes a page.
use super::material::{Bar, Edge, bar};
use super::space;
use recite_writer_session::palette::Palette;
use xilem::{
    WidgetView,
    core::one_of::Either,
    masonry::layout::AsUnit,
    peniko::Color,
    style::{Padding, Style},
    view::{CrossAxisAlignment, FlexExt, flex_col, flex_row, sized_box},
};

/// `message` and its `actions` as a banner `width` wide; `stacked` puts the actions on a second
/// line inside it.
pub(crate) fn banner<S: 'static, M: WidgetView<S>, A: WidgetView<S>>(
    palette: &Palette<Color>,
    (message, actions): (M, A),
    stacked: bool,
    width: f64,
) -> impl WidgetView<S> + use<S, M, A> {
    let body = if stacked {
        Either::A(
            flex_col((message, actions))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(space::SIBLINGS.px()),
        )
    } else {
        Either::B(
            flex_row((message.flex(1.), actions))
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(space::KINDS.px()),
        )
    };
    bar(
        Some(Bar::solid(palette.ink, Edge::Leading, 0., 0.)),
        sized_box(body)
            .padding(Padding::from_vh(space::SIBLINGS.px(), space::KINDS.px()))
            .background_color(palette.inset)
            .width(width.px()),
    )
}
