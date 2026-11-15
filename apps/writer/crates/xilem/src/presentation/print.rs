//! Print hierarchy and native action styles for Recite's writing surfaces.
use super::Type;
use super::control_style::{ControlStyle, styled};
use recite_writer_session::palette::Palette;
use xilem::peniko::Color;

mod separator;
pub(crate) use separator::separator;
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

/// A page or scene title: Display.
pub(crate) fn title(scale: f32, text: impl Into<ArcStr>) -> Label {
    Type::Display.label(scale, text)
}

/// Counts, hints and status sentences: Meta, in Plex Sans. Callers set the muted colour.
pub(crate) fn meta(scale: f32, text: impl Into<ArcStr>) -> Label {
    Type::Meta.label(scale, text)
}

/// A small tracked caps caption: the sidebar's scene list heading and source captions.
pub(crate) fn caps(scale: f32, text: impl Into<ArcStr>) -> Label {
    Type::Meta
        .label(scale, text)
        .weight(FontWeight::SEMI_BOLD)
        .letter_spacing(0.06 * Type::Meta.size(scale))
}

/// Ids, source labels and the folio: Plex Mono.
pub(crate) fn id(scale: f32, text: impl Into<ArcStr>) -> Label {
    Type::Id.label(scale, text)
}

pub(crate) fn action<S: 'static>(
    scale: f32,
    text: impl Into<ArcStr>,
    disabled: bool,
    callback: impl Fn(&mut S) + Send + Sync + 'static,
) -> impl WidgetView<S, Widget = Button> {
    styled(
        ControlStyle::Action,
        view::button(Type::Control.label(scale, text), callback).disabled(disabled),
    )
}

/// The one filled action on a surface. Its focus is a ring inside the fill, in the text colour.
pub(crate) fn primary_action<S: 'static, T: Into<ArcStr>, F: Fn(&mut S) + Send + Sync + 'static>(
    scale: f32,
    text: T,
    palette: &Palette<Color>,
    disabled: bool,
    callback: F,
) -> impl WidgetView<S> + use<S, T, F> {
    super::material::focus_ring(
        Some(palette.on_accent),
        styled(
            ControlStyle::Primary,
            view::button(
                Type::Control
                    .label(scale, text)
                    .weight(if disabled {
                        FontWeight::NORMAL
                    } else {
                        FontWeight::SEMI_BOLD
                    })
                    .color(if disabled {
                        palette.muted
                    } else {
                        palette.on_accent
                    }),
                callback,
            )
            .disabled(disabled),
        ),
    )
}

/// An action that is tonal at rest and becomes the view's primary while `primary` holds, such as
/// Save with unsaved changes. Both states share one widget, so focus survives the change, and the
/// accessible name never changes.
pub(crate) fn emphasised_action<
    S: 'static,
    T: Into<ArcStr>,
    F: Fn(&mut S) + Send + Sync + 'static,
>(
    scale: f32,
    text: T,
    palette: &Palette<Color>,
    primary: bool,
    disabled: bool,
    callback: F,
) -> impl WidgetView<S> + use<S, T, F> {
    let primary = primary && !disabled;
    super::material::focus_ring(
        primary.then_some(palette.on_accent),
        styled(
            if primary {
                ControlStyle::Primary
            } else {
                ControlStyle::Tonal
            },
            view::button(
                Type::Control
                    .label(scale, text)
                    .weight(if primary {
                        FontWeight::SEMI_BOLD
                    } else {
                        Type::Control.weight()
                    })
                    .color(if disabled {
                        palette.muted
                    } else if primary {
                        palette.on_accent
                    } else {
                        palette.ink
                    }),
                callback,
            )
            .disabled(disabled),
        ),
    )
}

/// The end of a beat: "END" between two colon devices in the second ink. The devices are
/// images named `◆`, so the announced mark is unchanged.
pub(crate) fn end_mark<S: 'static>(
    scale: f32,
    palette: &Palette<Color>,
) -> impl WidgetView<S> + use<S> {
    use xilem::{
        masonry::layout::{AsUnit, Dim},
        view::CrossAxisAlignment,
    };
    let colon = || super::icons::face(scale * 0.6, "◆", super::Icon::Colon, palette.teal);
    // Sized to its content, so a divider can draw its rules either side.
    view::sized_box(
        view::flex_row((colon(), id(scale, "END").color(palette.muted), colon()))
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap((super::space::UNIT * f64::from(scale)).px()),
    )
    .width(Dim::MaxContent)
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
    styled(
        ControlStyle::Icon,
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
        .disabled(disabled),
    )
}
