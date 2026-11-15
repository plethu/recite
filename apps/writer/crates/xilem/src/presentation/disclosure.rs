//! Disclosure state is a painted chevron beside the text, and the button that holds it reports
//! the native `expanded` state, so the chevron is decoration and no glyph enters the name.
use super::icons::{Icon, face};
use crate::presentation::space;
use xilem::{
    WidgetView,
    core::one_of::Either,
    masonry::layout::AsUnit,
    peniko::Color,
    style::Style,
    view::{CrossAxisAlignment, flex_row},
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
    flex_row((face(scale * MARK, "", icon, color), text))
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap((space::UNIT * f64::from(scale)).px())
}

/// A destination toggle: `→` while closed and a painted chevron while open. Its button is named
/// with the arrow in both states (`route_name`) and reports `expanded`.
pub(crate) fn route_toggle<S: 'static, V: WidgetView<S>>(
    scale: f32,
    expanded: bool,
    color: Color,
    text_size: f32,
    text: V,
) -> impl WidgetView<S> + use<S, V> {
    if expanded {
        Either::A(disclosure(scale, true, color, text))
    } else {
        Either::B(
            flex_row((
                super::label(1., "→").text_size(text_size).color(color),
                text,
            ))
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap((space::UNIT * f64::from(scale)).px()),
        )
    }
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
