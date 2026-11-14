//! Script is a clean sheet resting on the grained desk, or full-bleed paper where none fits.
use super::*;
use xilem::masonry::{properties::BoxShadow, widgets::Flex};
use xilem::peniko::Color;
use xilem::view::MainAxisAlignment;
use xilem::style::Padding;

/// Paper either side of the script measure, inside the sheet.
const MARGIN: f64 = 56.;
/// Desk visible above and below the sheet.
const DESK: f64 = 24.;
/// The hard offset of the sheet's shadow, right and down.
const SHADOW: f64 = 3.;
/// Below this window width, in logical pixels, Script is full-bleed.
pub(super) const MINIMUM_WINDOW: f64 = 900.;
/// The width a sheet adds to the script measure: margins, border and shadow.
pub(super) const EXTENT: f64 = 2. * MARGIN + 2. + SHADOW;

/// The Script pane's scrolled content. Both forms keep one widget tree, so switching between them
/// never replaces the mounted manuscript.
pub(super) fn paper<V: WidgetView<Writer>>(
    state: &Writer,
    layout: &Layout,
    manuscript: V,
) -> impl WidgetView<Writer, Widget = Flex> + use<V> {
    let sheet = layout.sheet;
    let paper = sized_box(manuscript)
        .padding(if sheet {
            Padding::from_vh(32.px(), MARGIN.px())
        } else {
            Padding::from_vh(16.px(), 0.px())
        })
        .background_color(state.palette.surface)
        .border_color(if sheet {
            state.palette.rule
        } else {
            Color::TRANSPARENT
        })
        .border_width(if sheet { 1.px() } else { 0.px() })
        .box_shadow(BoxShadow::new(
            if sheet {
                state.palette.stock
            } else {
                Color::TRANSPARENT
            },
            (SHADOW, SHADOW),
        ));
    flex_row((paper,))
        .main_axis_alignment(MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .padding(if sheet {
            Padding {
                top: DESK.px(),
                bottom: (DESK + SHADOW).px(),
                ..Padding::all(0.px())
            }
        } else {
            Padding::all(0.px())
        })
        .width(layout.script_pane.px())
}

/// The pane behind the paper: the desk around a sheet, or the paper itself when full-bleed.
pub(super) fn ground(state: &Writer, layout: &Layout) -> Color {
    if layout.sheet {
        Color::TRANSPARENT
    } else {
        state.palette.surface
    }
}

#[cfg(test)]
mod tests;
