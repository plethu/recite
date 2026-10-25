//! Print hierarchy and native action styles for Recite's writing surfaces.
use super::control_style::{ControlStyle, styled};
use super::label;
use recite_writer_session::palette::Palette;
use recite_writer_session::typography::{FONT_DISPLAY, FONT_MONO};
use xilem::peniko::Color;
use xilem::{
    WidgetView,
    masonry::{
        core::ArcStr,
        parley::{FontFamily, FontFamilyName, GenericFamily, style::FontWeight},
        widgets::Button,
    },
    style::Style,
    view::{self, Label},
};

pub(crate) fn font(families: &'static [&'static str]) -> FontFamily<'static> {
    FontFamily::List(
        families
            .iter()
            .map(|name| match *name {
                "serif" => GenericFamily::Serif.into(),
                "sans-serif" => GenericFamily::SansSerif.into(),
                "monospace" => GenericFamily::Monospace.into(),
                name => FontFamilyName::named(name),
            })
            .collect(),
    )
}

pub(crate) fn title(scale: f32, text: impl Into<ArcStr>) -> Label {
    label(scale, text)
        .font(font(FONT_DISPLAY))
        .text_size(32. * scale)
}

pub(crate) fn meta(scale: f32, text: impl Into<ArcStr>) -> Label {
    label(scale, text)
        .font(font(FONT_MONO))
        .text_size(12. * scale)
}

pub(crate) fn caps(scale: f32, text: impl Into<ArcStr>) -> Label {
    label(scale, text)
        .text_size(11. * scale)
        .weight(FontWeight::SEMI_BOLD)
        .letter_spacing(1.32 * scale)
}

pub(crate) fn action<S: 'static>(
    scale: f32,
    text: impl Into<ArcStr>,
    disabled: bool,
    callback: impl Fn(&mut S) + Send + Sync + 'static,
) -> impl WidgetView<S, Widget = Button> {
    styled(
        ControlStyle::Action,
        view::button(label(scale, text), callback).disabled(disabled),
    )
}

pub(crate) fn primary_action<S: 'static, T: Into<ArcStr>, F: Fn(&mut S) + Send + Sync + 'static>(
    scale: f32,
    text: T,
    palette: &Palette<Color>,
    disabled: bool,
    callback: F,
) -> impl WidgetView<S, Widget = Button> + use<S, T, F> {
    styled(
        ControlStyle::Primary,
        view::button(
            label(scale, text)
                .weight(FontWeight::SEMI_BOLD)
                .color(if disabled {
                    palette.muted
                } else {
                    palette.on_accent
                }),
            callback,
        )
        .disabled(disabled),
    )
}

/// Keep the ordinary native button and its name when the compact face is an icon.
pub(crate) fn icon_action<S: 'static, F: Fn(&mut S) + Send + Sync + 'static>(
    scale: f32,
    name: &'static str,
    icon: super::Icon,
    palette: &Palette<Color>,
    disabled: bool,
    callback: F,
) -> impl WidgetView<S, Widget = Button> + use<S, F> {
    use xilem::{masonry::layout::AsUnit, style::Style};
    styled(
        ControlStyle::Action,
        view::button(
            super::icons::face(
                scale,
                name,
                icon,
                if disabled {
                    palette.muted.with_alpha(0.4)
                } else {
                    palette.ink
                },
            ),
            callback,
        )
        .disabled(disabled)
        .padding(xilem::style::Padding::all((4. * f64::from(scale)).px())),
    )
}
