//! Actions with an icon face: icon actions and toggles, the text toggle with its tick, and the
//! insertion affordance led by a plus.
use super::super::control_style::{ControlStyle, styled};
use super::super::{Icon, Type, icons, space};
use recite_writer_session::palette::Palette;
use xilem::peniko::Color;
use xilem::{
    WidgetView,
    masonry::{layout::AsUnit, widgets::Button},
    style::Style,
    view,
};

/// An icon action's width: its face and the Icon style's padding and 1px border either side.
pub(crate) fn icon_action_width(scale: f32) -> f64 {
    (icons::FACE + 2. * space::UNIT) * f64::from(scale) + 2.
}

/// An icon toggle such as "Pin reference": the same compact face, reporting `pressed` and held
/// in the pressed sink while on.
pub(crate) fn icon_toggle<S: 'static, F: Fn(&mut S) + Send + Sync + 'static>(
    scale: f32,
    (name, icon): (&'static str, Icon),
    palette: &Palette<Color>,
    (pressed, disabled): (bool, bool),
    callback: F,
) -> impl WidgetView<S, Widget = Button> + use<S, F> {
    styled(
        if pressed {
            ControlStyle::IconOn
        } else {
            ControlStyle::Icon
        },
        view::button(icon_face(scale, name, icon, palette, disabled), callback)
            .toggled(pressed)
            .disabled(disabled),
    )
}

/// A margin toggle with words, such as "Reviewed": named in full, reporting `pressed`, and while on
/// held in the pressed sink with a tick before its words, so the state shows by shape as well.
pub(crate) fn text_toggle<S: 'static, F: Fn(&mut S) + Send + Sync + 'static>(
    scale: f32,
    (name, text): (&'static str, &'static str),
    palette: &Palette<Color>,
    (pressed, disabled): (bool, bool),
    callback: F,
) -> impl WidgetView<S, Widget = Button> + use<S, F> {
    styled(
        if pressed {
            ControlStyle::ToggleOn
        } else {
            ControlStyle::Tonal
        },
        view::button(
            view::flex_row((
                pressed
                    .then(|| icon_face(scale * icons::SMALL, "", Icon::Check, palette, disabled)),
                Type::Control.label(scale, text),
            ))
            .gap((space::UNIT * f64::from(scale)).px()),
            callback,
        )
        .accessible_label(name)
        .toggled(pressed)
        .disabled(disabled),
    )
}

/// How far an insertion affordance's words sit from its start: the plus and its gap, so the
/// plus can hang before a column and the words line up with it.
pub(crate) fn insert_hang(scale: f32) -> f64 {
    (icons::FACE * f64::from(icons::SMALL) + space::UNIT) * f64::from(scale)
}

/// An insertion affordance: a quiet action led by a plus, such as "+ Line" after the last row,
/// named in full for assistive technology ("Add line").
pub(crate) fn insert_action<S: 'static, F: Fn(&mut S) + Send + Sync + 'static>(
    scale: f32,
    (name, text): (&'static str, &'static str),
    palette: &Palette<Color>,
    disabled: bool,
    callback: F,
) -> impl WidgetView<S, Widget = Button> + use<S, F> {
    styled(
        ControlStyle::Action,
        view::button(
            view::flex_row((
                icon_face(scale * icons::SMALL, "", Icon::Plus, palette, disabled),
                Type::Control.label(scale, text),
            ))
            .gap((space::UNIT * f64::from(scale)).px()),
            callback,
        )
        .accessible_label(name)
        .disabled(disabled),
    )
}

fn icon_face(
    scale: f32,
    name: &'static str,
    icon: Icon,
    palette: &Palette<Color>,
    disabled: bool,
) -> icons::Face {
    icons::face(
        scale,
        name,
        icon,
        if disabled {
            palette.muted.with_alpha(0.4)
        } else {
            palette.ink
        },
    )
}

/// Keep the ordinary native button and its name when the compact face is an icon.
pub(crate) fn icon_action<S: 'static, F: Fn(&mut S) + Send + Sync + 'static>(
    scale: f32,
    name: &'static str,
    icon: Icon,
    palette: &Palette<Color>,
    disabled: bool,
    callback: F,
) -> impl WidgetView<S, Widget = Button> + use<S, F> {
    styled(
        ControlStyle::Icon,
        view::button(icon_face(scale, name, icon, palette, disabled), callback).disabled(disabled),
    )
}
