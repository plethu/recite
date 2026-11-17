//! Print hierarchy and native action styles for Recite's writing surfaces.
use super::Type;
use super::control_style::{ControlStyle, styled};
use super::group::{Named, heading};
use crate::app::keyboard::ReturnFocus;
use recite_writer_session::palette::Palette;
use xilem::peniko::Color;

mod icon_actions;
mod separator;
mod title;
pub(crate) use icon_actions::{
    icon_action, icon_action_width, icon_toggle, insert_action, insert_hang, text_toggle,
};
pub(crate) use separator::separator;
pub(crate) use title::Title;
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

/// A page's or scene's title in the Heading role at level 1.
pub(crate) fn page_heading(scale: f32, text: impl Into<ArcStr>) -> Named<Label> {
    let text = text.into();
    heading(1, text.clone(), title(scale, text))
}

/// A section's head in Section type and the Heading role at level 2.
pub(crate) fn section_heading(scale: f32, text: impl Into<ArcStr>) -> Named<Label> {
    let text = text.into();
    heading(2, text.clone(), Type::Section.label(scale, text))
}

/// A sheet's or dialog's caption and guidance under a field or list: Body, in Plex Sans, so a
/// sentence a writer reads is never set smaller than body text. Callers set the muted colour.
pub(crate) fn caption(scale: f32, text: impl Into<ArcStr>) -> Label {
    Type::Body.label(scale, text)
}

/// Counts, shortcuts and key hints: Meta, in Plex Sans. Callers set the muted colour.
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

/// A quiet action whose name says more than its words, such as which row it opens.
pub(crate) fn action_named<S: 'static>(
    scale: f32,
    (text, name): (impl Into<ArcStr>, impl Into<ArcStr>),
    disabled: bool,
    callback: impl Fn(&mut S) + Send + Sync + 'static,
) -> impl WidgetView<S, Widget = Button> {
    styled(
        ControlStyle::Action,
        view::button(Type::Control.label(scale, text), callback)
            .accessible_label(name.into())
            .disabled(disabled),
    )
}

/// A quiet action that shows or hides a panel beneath its row, its words shorter than its name
/// and its state reported as expanded, such as Context in the preview's head.
pub(crate) fn action_disclosing<S: 'static>(
    scale: f32,
    (text, name): (&'static str, &'static str),
    (expanded, disabled): (bool, bool),
    callback: impl Fn(&mut S) + Send + Sync + 'static,
) -> impl WidgetView<S, Widget = Button> {
    styled(
        ControlStyle::Action,
        view::button(Type::Control.label(scale, text), callback)
            .accessible_label(name)
            .expanded(expanded)
            .disabled(disabled),
    )
}

/// The one filled action on a surface. Its focus is a ring inside the fill, in the text colour.
pub(crate) fn primary_action<S: 'static, T: Into<Title>, F: Fn(&mut S) + Send + Sync + 'static>(
    scale: f32,
    text: T,
    palette: &Palette<Color>,
    disabled: bool,
    callback: F,
) -> impl WidgetView<S> + use<S, T, F> {
    primary_returning(scale, text, palette, (disabled, false), callback)
}

/// [`primary_action`] that takes focus back when the control holding it goes while `returning`,
/// such as Continue preview after a step.
pub(crate) fn primary_returning<
    S: 'static,
    T: Into<Title>,
    F: Fn(&mut S) + Send + Sync + 'static,
>(
    scale: f32,
    text: T,
    palette: &Palette<Color>,
    (disabled, returning): (bool, bool),
    callback: F,
) -> impl WidgetView<S> + use<S, T, F> {
    primary_described(scale, text, palette, (disabled, returning), None, callback)
}

/// [`primary_returning`] whose accessible description says why it waits, such as what a
/// proposal still lacks, the same words the view sets beside the field they concern.
pub(crate) fn primary_described<
    S: 'static,
    T: Into<Title>,
    F: Fn(&mut S) + Send + Sync + 'static,
>(
    scale: f32,
    text: T,
    palette: &Palette<Color>,
    (disabled, returning): (bool, bool),
    description: Option<&'static str>,
    callback: F,
) -> impl WidgetView<S> + use<S, T, F> {
    let title = text.into();
    let button = view::button(
        // One weight in both states, so the words keep their width and nothing beside the action
        // moves as it becomes available.
        Type::Control
            .label(scale, title.face)
            .weight(FontWeight::SEMI_BOLD)
            .color(if disabled {
                palette.muted
            } else {
                palette.on_accent
            }),
        callback,
    );
    let button = match title.name {
        Some(name) => button.accessible_label(name),
        None => button,
    };
    let button = match description {
        Some(description) => button.accessible_description(description),
        None => button,
    };
    super::material::focus_ring(
        Some(palette.on_accent),
        styled(
            ControlStyle::Primary,
            button.disabled(disabled).prop(ReturnFocus(returning)),
        ),
    )
}

/// An action that rests in `rest`, tonal or quiet, and becomes the view's primary while `primary`
/// holds, such as Save with unsaved changes. Both states share one widget, so focus survives the change, and the
/// accessible name never changes.
pub(crate) fn emphasised_action<
    S: 'static,
    T: Into<Title>,
    F: Fn(&mut S) + Send + Sync + 'static,
>(
    scale: f32,
    text: T,
    palette: &Palette<Color>,
    (primary, rest): (bool, ControlStyle),
    disabled: bool,
    callback: F,
) -> impl WidgetView<S> + use<S, T, F> {
    let primary = primary && !disabled;
    let title = text.into();
    let button = view::button(
        // One weight in both states, so the words keep their width and nothing beside the action
        // moves as it becomes primary; the fill carries the emphasis.
        Type::Control
            .label(scale, title.face)
            .weight(Type::Control.weight())
            .color(if disabled {
                palette.muted
            } else if primary {
                palette.on_accent
            } else {
                palette.ink
            }),
        callback,
    );
    let button = match title.name {
        Some(name) => button.accessible_label(name),
        None => button,
    };
    super::material::focus_ring(
        primary.then_some(palette.on_accent),
        styled(
            if primary { ControlStyle::Primary } else { rest },
            button.disabled(disabled),
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

/// A 1px rule in the rule ink across its container: under a running head, over a running foot,
/// between beats.
pub(crate) fn hairline<S: 'static>(palette: &Palette<Color>) -> impl WidgetView<S> + use<S> {
    use xilem::masonry::{layout::AsUnit, properties::ContentColor};
    view::divider_h()
        .thickness(1.px())
        .prop(ContentColor::new(palette.rule))
}

/// The dotted rule that ends a beat, `width` wide with the end mark at its centre.
pub(crate) fn end_rule<S: 'static>(
    scale: f32,
    palette: &Palette<Color>,
    width: f64,
) -> impl WidgetView<S> + use<S> {
    use xilem::masonry::{kurbo::Axis, layout::AsUnit, properties::ContentColor};
    view::divider(Axis::Horizontal, end_mark(scale, palette))
        .dash_pattern(&[1.px(), 3.px()])
        .prop(ContentColor::new(palette.rule))
        .width(width.px())
}
