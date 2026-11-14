//! A segmented choice: one ink frame around flush native radio segments, the chosen one a solid
//! stamp whose keyboard focus is a ring in its own text colour.
use super::control_style::{ControlStyle, styled};
use super::material::focus_ring;
use super::radio_button;
use recite_writer_session::palette::Palette;
use xilem::{
    WidgetView,
    masonry::{
        core::ArcStr,
        layout::{AsUnit, Dim},
        properties::CheckmarkSize,
    },
    peniko::Color,
    style::Style,
    view::{FlexSequence, flex_row, radio_group, sized_box},
};

pub(crate) fn segment<S: 'static, T: Into<ArcStr>, F: Fn(&mut S) + Send + Sync + 'static>(
    scale: f32,
    palette: &Palette<Color>,
    title: T,
    selected: bool,
    disabled: bool,
    select: F,
) -> impl WidgetView<S> + use<S, T, F> {
    focus_ring(
        selected.then_some(palette.on_stamp),
        styled(
            if selected {
                ControlStyle::Selected
            } else {
                ControlStyle::Segment
            },
            radio_button(scale, title, selected, select)
                .label_color(if selected {
                    palette.on_stamp
                } else if disabled {
                    palette.muted
                } else {
                    palette.ink
                })
                .disabled(disabled)
                .prop(CheckmarkSize {
                    side: 0.px(),
                    gap: 0.px(),
                }),
        ),
    )
}

pub(crate) fn segments<S: 'static, Seq: FlexSequence<S> + Send + Sync>(
    palette: &Palette<Color>,
    choices: Seq,
) -> impl WidgetView<S> + use<S, Seq> {
    sized_box(radio_group(flex_row(choices).gap(0.px())))
        .width(Dim::MaxContent)
        .border_color(palette.ink)
        .border_width(1.px())
}
