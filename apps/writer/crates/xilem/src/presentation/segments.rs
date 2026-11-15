//! Mode switches. Both are one native radio group, so each is one Tab stop with arrow keys, and the
//! choice carries two cues besides colour.
//!
//! - **Segments** (Script | Map | Split): one tonal track; the chosen chip sits on the reading
//!   surface in semibold with a 2px ink bar under its label.
//! - **Tabs** (Write | Localise): quiet text; the chosen tab is semibold over a 2px underline in
//!   the second ink, which holds 3:1 on the grained desk.
use super::control_style::{ControlStyle, styled};
use super::material::{Bar, bar};
use super::{Type, radio_button, space};
use recite_writer_session::palette::{Palette, TONAL};
use xilem::{
    WidgetView,
    masonry::{
        core::ArcStr,
        kurbo::Axis,
        layout::{AsUnit, Dim},
        parley::style::FontWeight,
        properties::CheckmarkSize,
        widgets::RadioButton,
    },
    peniko::Color,
    style::Style,
    view::{CrossAxisAlignment, FlexSequence, flex_row, radio_group, sized_box},
};

/// How far a chosen segment's bar sits above the chip's bottom edge.
const LIFT: f64 = 4.;

fn choice<S: 'static, T: Into<ArcStr>, F: Fn(&mut S) + Send + Sync + 'static>(
    scale: f32,
    palette: &Palette<Color>,
    title: T,
    selected: bool,
    disabled: bool,
    select: F,
) -> impl WidgetView<S, Widget = RadioButton> + use<S, T, F> {
    radio_button(scale, title, selected, select)
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
    bar(
        selected.then_some(Bar {
            color: palette.ink,
            inset: 12. * f64::from(scale),
            lift: LIFT * f64::from(scale),
        }),
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

pub(crate) fn tab<S: 'static, T: Into<ArcStr>, F: Fn(&mut S) + Send + Sync + 'static>(
    scale: f32,
    palette: &Palette<Color>,
    title: T,
    selected: bool,
    disabled: bool,
    select: F,
) -> impl WidgetView<S> + use<S, T, F> {
    bar(
        selected.then_some(Bar {
            color: palette.teal,
            inset: 0.,
            lift: 0.,
        }),
        styled(
            ControlStyle::Tab,
            choice(scale, palette, title, selected, disabled, select),
        ),
    )
}

/// Text tabs, a sibling gap apart.
pub(crate) fn tabs<S: 'static, Seq: FlexSequence<S> + Send + Sync>(
    choices: Seq,
) -> impl WidgetView<S> + use<S, Seq> {
    sized_box(radio_group(flex_row(choices).gap(space::UNIT.px()))).width(Dim::MaxContent)
}
