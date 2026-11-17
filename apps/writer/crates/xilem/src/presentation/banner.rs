//! Banners: an errata slip tipped in at the top of the sheet it concerns. Floating paper closed by
//! a 1px boundary edge, with a hard stock offset like a map card's, and no side bar; a plum caps
//! lead-in says what kind of notice it is, then the message and its actions follow. The message
//! keeps one line and the actions follow it, or take a second line inside the slip when the row is
//! too narrow for both; a banner never becomes a page. The edge and the offset come out of the
//! padding, so a banner is as tall as its words and padding were on inset paper.
#[cfg(test)]
mod tests;
use super::material::{Decorated, Decoration};
use super::{print, space};
use recite_writer_session::palette::Palette;
use xilem::{
    WidgetView,
    core::one_of::Either,
    masonry::layout::AsUnit,
    peniko::Color,
    style::{Padding, Style},
    view::{CrossAxisAlignment, FlexExt, flex_col, flex_row, sized_box},
};

/// The slip's hard stock offset, right and down.
pub(crate) const STOCK: f64 = 3.;
/// The slip's closed edge.
const EDGE: f64 = 1.;

/// `message` and its `actions` as a banner `width` wide, led by `lead` in caps at `scale`;
/// `stacked` puts the actions on a second line inside it.
pub(crate) fn banner<S: 'static, M: WidgetView<S>, A: WidgetView<S>>(
    palette: &Palette<Color>,
    (scale, lead): (f32, &'static str),
    (message, actions): (M, A),
    stacked: bool,
    width: f64,
) -> impl WidgetView<S> + use<S, M, A> {
    // The lead-in shares the message's first baseline, whether the message is one line or more.
    let message = flex_row((
        print::caps(scale, lead.to_uppercase()).color(palette.accent),
        message.flex(1.),
    ))
    .cross_axis_alignment(CrossAxisAlignment::FirstBaseline)
    .gap(space::SIBLINGS.px());
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
    let (vertical, horizontal) = (space::SIBLINGS - EDGE, space::KINDS - EDGE);
    Decorated::new(
        Decoration::Stock(palette.stock, STOCK),
        sized_box(
            sized_box(body)
                .padding(Padding {
                    top: vertical.px(),
                    bottom: (vertical - STOCK).px(),
                    left: horizontal.px(),
                    right: (horizontal - STOCK).px(),
                })
                .background_color(palette.floating)
                .border_color(palette.boundary)
                .border_width(EDGE.px()),
        )
        .padding(Padding {
            right: STOCK.px(),
            bottom: STOCK.px(),
            ..Padding::all(0.px())
        })
        .width(width.px()),
    )
}
