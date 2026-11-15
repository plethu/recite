//! A search field: a tonal well whose leading magnifier is painted beneath the input, so the whole
//! well, icon included, takes the pointer and focus while the icon stays out of the accessible text.
use super::control_style::{ControlStyle, styled};
use super::icons::{Icon, face};
use xilem::{
    WidgetView,
    masonry::{
        layout::{AsUnit, UnitPoint},
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

/// Wrap `input` as a search field on paper or, with `desk`, on the grained desk.
pub(crate) fn search_field<S: 'static, V: WidgetView<S, Widget = TextInput>>(
    scale: f32,
    ink: Color,
    desk: bool,
    input: V,
) -> impl WidgetView<S> + use<S, V> {
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
                input,
            ),
        ),
    ))
    .alignment(UnitPoint::LEFT)
}
