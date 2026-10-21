//! Print hierarchy and native action styles for Recite's writing surfaces.
use super::control_style::{ControlStyle, styled};
use super::label;
use recite_writer_session::typography::{FONT_DISPLAY, FONT_MONO};
use xilem::{
    WidgetView,
    masonry::{
        core::ArcStr,
        parley::{FontFamily, FontFamilyName, GenericFamily, style::FontWeight},
        widgets::Button,
    },
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
