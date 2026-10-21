//! The specimen composes the same buttons, fields, palette and manuscript text as Writer.
mod inputs;
mod samples;
use super::*;
use crate::presentation::{label, paragraph, text_button};
use xilem::masonry::properties::LineBreaking;
use xilem::{
    WidgetView,
    masonry::layout::AsUnit,
    style::Style,
    view::{CrossAxisAlignment, checkbox, flex_col, flex_row, portal, sized_box},
};

pub(super) fn specimen(state: &Specimen) -> impl WidgetView<Specimen> + use<> {
    portal(
        sized_box(
            flex_col((
                header(state),
                paragraph(state.scale(), state.message.clone(), 640.),
                actions(state),
                inputs::inputs(state),
                samples::samples(state),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(24.px()),
        )
        .padding(24.px())
        .background_color(state.theme.palette.canvas),
    )
    .constrain_horizontal(true)
}
fn header(state: &Specimen) -> impl WidgetView<Specimen> + use<> {
    flex_col((
        flex_row((label(state.scale(), "Recite · native component specimen")
            .text_size(28. * state.scale())
            .prop(LineBreaking::WordWrap)
            .width(640.px()),)),
        paragraph(
            state.scale(),
            "Production controls and typography. Changes here last only for this window.",
            640.,
        ),
        flex_row((
            text_button(
                state.scale(),
                if state.config.theme == WriterTheme::Dark {
                    "Light appearance"
                } else {
                    "Dark appearance"
                },
                Specimen::appearance,
            ),
            text_button(state.scale(), "Monochrome", Specimen::monochrome),
        ))
        .gap(8.px()),
        flex_row((
            text_button(
                state.scale(),
                if state.scale() == 1. {
                    "200% UI scale"
                } else {
                    "100% UI scale"
                },
                Specimen::scale_interface,
            ),
            text_button(state.scale(), "Close specimen", |state: &mut Specimen| {
                state.closed = true
            }),
        ))
        .gap(8.px()),
        checkbox(
            "Reduced motion",
            state.config.reduced_motion,
            |state: &mut Specimen, checked| state.config.reduced_motion = checked,
        )
        .text_size(14. * state.scale()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}
fn actions(state: &Specimen) -> impl WidgetView<Specimen> + use<> {
    flex_col((
        label(state.scale(), "Actions and navigation").text_size(22. * state.scale()),
        flex_col((
            text_button(state.scale(), "Try scene", |state: &mut Specimen| {
                state.message = "Primary action activated.".into()
            }),
            text_button(state.scale(), "Add beat", |state: &mut Specimen| {
                state.message = "Secondary action activated.".into()
            }),
            text_button(
                state.scale(),
                "Arrange automatically",
                |state: &mut Specimen| state.message = "Quiet action activated.".into(),
            ),
            text_button(state.scale(), "Save changes", |_: &mut Specimen| {}).disabled(true),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}
