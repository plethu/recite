//! Mode switches. Each is one Tab stop with arrow keys, and the choice carries two cues besides
//! colour.
//!
//! - **Segments** (Script | Map | Split and every other one-of choice): one tonal track; the
//!   chosen chip is floating paper in semibold with a 2px ink bar under its label.
//! - **Tabs** (Write | Localise): native tabs in a tab list with roving focus, quiet text; the
//!   chosen tab is semibold over a 2px underline in the second ink, which holds 3:1 on the grained
//!   desk.
use super::control_style::{ControlStyle, styled};
use super::group::tab_list;
use super::material::{Bar, Edge, bar};
use super::{Type, radio_button, space};
use recite_writer_session::palette::{Palette, TONAL};
use xilem::{
    WidgetView,
    masonry::{
        accesskit::Role,
        core::ArcStr,
        kurbo::Axis,
        layout::{AsUnit, Dim},
        parley::style::FontWeight,
        properties::CheckmarkSize,
        widgets::RadioButton,
    },
    peniko::Color,
    style::Style,
    view::{CrossAxisAlignment, FlexSequence, button, flex_row, radio_group, sized_box},
};

/// How far a chosen segment's bar sits above the chip's bottom edge.
const LIFT: f64 = 4.;

/// One choice: its visible face, and the full name read in its place when the face is short.
struct Title {
    face: ArcStr,
    name: Option<ArcStr>,
}
impl<T: Into<ArcStr>> From<T> for Title {
    fn from(face: T) -> Self {
        Self {
            face: face.into(),
            name: None,
        }
    }
}

fn choice<S: 'static, F: Fn(&mut S) + Send + Sync + 'static>(
    scale: f32,
    palette: &Palette<Color>,
    title: Title,
    selected: bool,
    disabled: bool,
    select: F,
) -> impl WidgetView<S, Widget = RadioButton> + use<S, F> {
    radio_button(scale, title.face, selected, select)
        .accessible_label(title.name)
        .label_color(if disabled { palette.muted } else { palette.ink })
        .weight(if selected {
            FontWeight::SEMI_BOLD
        } else {
            Type::Control.weight()
        })
        .disabled(disabled)
        .prop(CheckmarkSize {
            side: 0.px(),
            gap: 0.px(),
        })
}

pub(crate) fn segment<S: 'static, T: Into<ArcStr>, F: Fn(&mut S) + Send + Sync + 'static>(
    scale: f32,
    palette: &Palette<Color>,
    title: T,
    selected: bool,
    disabled: bool,
    select: F,
) -> impl WidgetView<S> + use<S, T, F> {
    chip(
        scale,
        palette,
        Title::from(title),
        selected,
        disabled,
        select,
    )
}

/// A segment showing the short `face` ("Light") and read by its full `name` ("Light theme"), for
/// a choice whose row label already says what is chosen.
pub(crate) fn segment_named<
    S: 'static,
    T: Into<ArcStr>,
    N: Into<ArcStr>,
    F: Fn(&mut S) + Send + Sync + 'static,
>(
    scale: f32,
    palette: &Palette<Color>,
    title: (T, N),
    selected: bool,
    select: F,
) -> impl WidgetView<S> + use<S, T, N, F> {
    segment_titled(scale, palette, title, selected, false, select)
}

/// [`segment_named`] that can be unavailable, such as a choice repeated on every row of a list.
pub(crate) fn segment_titled<
    S: 'static,
    T: Into<ArcStr>,
    N: Into<ArcStr>,
    F: Fn(&mut S) + Send + Sync + 'static,
>(
    scale: f32,
    palette: &Palette<Color>,
    (face, name): (T, N),
    selected: bool,
    disabled: bool,
    select: F,
) -> impl WidgetView<S> + use<S, T, N, F> {
    chip(
        scale,
        palette,
        Title {
            face: face.into(),
            name: Some(name.into()),
        },
        selected,
        disabled,
        select,
    )
}

fn chip<S: 'static, F: Fn(&mut S) + Send + Sync + 'static>(
    scale: f32,
    palette: &Palette<Color>,
    title: Title,
    selected: bool,
    disabled: bool,
    select: F,
) -> impl WidgetView<S> + use<S, F> {
    bar(
        selected.then_some(Bar::solid(
            palette.ink,
            Edge::Bottom,
            12. * f64::from(scale),
            LIFT * f64::from(scale),
        )),
        styled(
            if selected {
                ControlStyle::Chosen
            } else {
                ControlStyle::Segment
            },
            choice(scale, palette, title, selected, disabled, select),
        ),
    )
}

/// One tonal track around flush segments.
pub(crate) fn segments<S: 'static, Seq: FlexSequence<S> + Send + Sync>(
    palette: &Palette<Color>,
    choices: Seq,
) -> impl WidgetView<S> + use<S, Seq> {
    segments_along(palette, Axis::Horizontal, choices)
}

/// One tonal track around flush segments laid along `axis`, for windows too narrow for a row.
pub(crate) fn segments_along<S: 'static, Seq: FlexSequence<S> + Send + Sync>(
    palette: &Palette<Color>,
    axis: Axis,
    choices: Seq,
) -> impl WidgetView<S> + use<S, Seq> {
    sized_box(radio_group(
        flex_row(choices)
            .direction(axis)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(0.px()),
    ))
    .width(Dim::MaxContent)
    .background_color(palette.ink.with_alpha(TONAL))
    .corner_radius(2.px())
}

/// A text tab: a native tab whose name is its text, selected while chosen. Only the selected tab
/// is a Tab stop; Left and Right reach the others (roving focus).
pub(crate) fn tab<S: 'static, F: Fn(&mut S) + Send + Sync + 'static>(
    scale: f32,
    palette: &Palette<Color>,
    title: &'static str,
    selected: bool,
    disabled: bool,
    select: F,
) -> impl WidgetView<S> + use<S, F> {
    bar(
        selected.then_some(Bar::solid(palette.teal, Edge::Bottom, 0., 0.)),
        styled(
            ControlStyle::Tab,
            button(
                Type::Control
                    .label(scale, title)
                    .weight(if selected {
                        FontWeight::SEMI_BOLD
                    } else {
                        Type::Control.weight()
                    })
                    .color(if disabled { palette.muted } else { palette.ink }),
                select,
            )
            .role(Role::Tab)
            .accessible_label(title)
            .selected(selected)
            .focusable(selected)
            .disabled(disabled),
        ),
    )
}

/// Text tabs, a sibling gap apart, in one tab list named `name`.
pub(crate) fn tabs<S: 'static, Seq: FlexSequence<S> + Send + Sync>(
    name: &'static str,
    choices: Seq,
) -> impl WidgetView<S> + use<S, Seq> {
    sized_box(tab_list(name, flex_row(choices).gap(space::UNIT.px()))).width(Dim::MaxContent)
}
