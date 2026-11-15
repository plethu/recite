//! A search field: a tonal well whose leading magnifier is painted beneath the input, so the whole
//! well, icon included, takes the pointer and focus while the icon stays out of the accessible text.
use super::control_style::{ControlStyle, styled};
use super::icons::{Icon, face};
use xilem::{
    WidgetView,
    masonry::{
        layout::{AsUnit, Dim, UnitPoint},
        widgets::TextInput,
    },
    peniko::Color,
    style::{Padding, Style},
    view::{sized_box, zstack},
};

/// Room the field's text leaves for the icon, beyond its usual padding.
pub(crate) const LEAD: f64 = 20.;
/// Where the icon starts inside the well.
const INSET: f64 = 10.;
/// The icon's size relative to the 20px icon faces.
const MARK: f32 = 0.8;

/// A search field's height: a row and a pixel, so its text line, the field's accessible bounds,
/// stays 20px tall as it was when toolbars stretched the field.
const HEIGHT: f64 = super::tokens::size::ROW + 1.;

/// Wrap `input` as a search field on paper or, with `desk`, on the grained desk, `width` wide when
/// given. The field sets the input's dimensions, so callers do not size `input` themselves.
pub(crate) fn search_field<S: 'static, V: WidgetView<S, Widget = TextInput>>(
    scale: f32,
    ink: Color,
    desk: bool,
    width: Option<f64>,
    input: V,
) -> impl WidgetView<S> + use<S, V> {
    let height = (HEIGHT * f64::from(scale)).px();
    zstack((
        sized_box(face(scale * MARK, "", Icon::Search, ink))
            .padding(Padding::left((INSET * f64::from(scale)).px())),
        styled(
            ControlStyle::SearchField,
            styled(
                if desk {
                    ControlStyle::DeskField
                } else {
                    ControlStyle::Field
                },
                input.dims((
                    width.map_or(Dim::Auto, |width| Dim::Fixed(width.px())),
                    Dim::Fixed(height),
                )),
            ),
        ),
    ))
    .alignment(UnitPoint::LEFT)
}
