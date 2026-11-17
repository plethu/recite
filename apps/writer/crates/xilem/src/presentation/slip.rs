//! Slips: menus, popovers and modal panels printed on floating paper with a 1px ink edge and a
//! hard stock offset, so they read as a separate sheet laid on the work, never as a soft card.
use super::material::{Decorated, Decoration};
use recite_writer_session::palette::Palette;
use xilem::{
    WidgetView,
    masonry::layout::AsUnit,
    peniko::Color,
    style::{Padding, Style},
    view::sized_box,
};

/// The hard offset of a slip's stock, right and down.
pub(crate) const OFFSET: f64 = 4.;

/// `child` on a slip of floating paper, its stock showing `OFFSET` right of and below it.
pub(crate) fn slip<S: 'static, V: WidgetView<S>>(
    palette: &Palette<Color>,
    child: V,
) -> impl WidgetView<S> + use<S, V> {
    Decorated::new(
        Decoration::Stock(palette.stock, OFFSET),
        sized_box(
            sized_box(child)
                .background_color(palette.floating)
                .border_color(palette.ink)
                .border_width(1.px()),
        )
        .padding(Padding {
            right: OFFSET.px(),
            bottom: OFFSET.px(),
            ..Padding::all(0.px())
        }),
    )
}
