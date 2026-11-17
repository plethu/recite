//! Disclosure state is a painted chevron beside the text, and the button that holds it reports
//! the native `expanded` state, so the chevron is decoration and no glyph enters the name.
use super::icons::{Icon, face};
use crate::presentation::{Type, space};
use xilem::{
    WidgetView,
    masonry::layout::AsUnit,
    peniko::Color,
    style::Style,
    view::{CrossAxisAlignment, FlexExt, MainAxisAlignment, flex_row, sized_box},
};

/// The chevron's size relative to 20px icon faces, to sit with 14px interface text.
const MARK: f32 = 0.75;

pub(crate) fn disclosure<S: 'static, V: WidgetView<S>>(
    scale: f32,
    expanded: bool,
    color: Color,
    text: V,
) -> impl WidgetView<S> + use<S, V> {
    let icon = if expanded {
        Icon::Expanded
    } else {
        Icon::Collapsed
    };
    // The words take what the row leaves them, so a one-line excerpt abbreviates within it.
    flex_row((face(scale * MARK, "", icon, color), text.flex(1.)))
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap((space::UNIT * f64::from(scale)).px())
}

/// A destination toggle: `→` while closed and a painted chevron while open. Its button is named
/// with the arrow in both states (`route_name`) and reports `expanded`.
///
/// The destination is set in the manuscript's type, which does not grow with the UI scale; a
/// strut a Control line tall at `ui_scale` keeps the toggle at least a text control's row tall,
/// so its hit target grows with the interface like every other control's. The strut comes
/// after the face, so the toggle's first baseline is its words' and a row aligns it by them.
pub(crate) fn route_toggle<S: 'static, V: WidgetView<S>>(
    (scale, ui_scale): (f32, f32),
    expanded: bool,
    color: Color,
    text_size: f32,
    text: V,
) -> impl WidgetView<S> + use<S, V> {
    // Options, not one Either, whose wrapper would hide the words' baseline from the row.
    let face = flex_row((
        expanded.then(|| face(scale * MARK, "", Icon::Expanded, color)),
        (!expanded).then(|| super::label(1., "→").text_size(text_size).color(color)),
        text,
    ))
    .cross_axis_alignment(CrossAxisAlignment::FirstBaseline)
    .gap((space::UNIT * f64::from(scale)).px());
    let line = f64::from(Type::Control.metrics().1 * ui_scale);
    flex_row((
        face,
        sized_box(flex_row(())).width(0.px()).height(line.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(0.px())
}

/// A label that opens a choice, such as the speaker of the line being written: a down chevron
/// before its words says it is a control. The chevron leads, so right-aligned words keep their
/// place. It is decoration; the control names itself.
pub(crate) fn chooser<S: 'static, V: WidgetView<S>>(
    scale: f32,
    color: Color,
    text: V,
) -> impl WidgetView<S> + use<S, V> {
    flex_row((face(scale * MARK, "", Icon::Expanded, color), text))
        .main_axis_alignment(MainAxisAlignment::End)
        // The words' baseline is the row's, so the control still aligns by the label's.
        .cross_axis_alignment(CrossAxisAlignment::FirstBaseline)
        .gap((space::UNIT * f64::from(scale)).px())
}

/// A destination toggle's name in both states: the arrow and the destination's display name.
pub(crate) fn route_name(destination: &str) -> String {
    format!("→ {destination}")
}

/// The width the chevron and its gap take from a row of text.
pub(crate) fn disclosure_width(scale: f32) -> f64 {
    f64::from(scale) * (20. * f64::from(MARK) + 4.)
}

#[cfg(test)]
mod tests;
