//! The specimen composes the same buttons, fields, palette and manuscript text as Writer.
use crate::presentation::space;
use crate::presentation::{Type, Typeset};
mod inputs;
mod samples;
use super::*;
use crate::presentation::{label, material::desk, paragraph, print, text_button};
use xilem::masonry::properties::LineBreaking;
use xilem::{
    WidgetView,
    masonry::layout::AsUnit,
    style::Style,
    view::{CrossAxisAlignment, checkbox, flex_col, flex_row, portal, sized_box},
};

pub(super) fn specimen(state: &Specimen) -> impl WidgetView<Specimen> + use<> {
    let page = portal(
        sized_box(
            flex_col((
                header(state),
                paragraph(state.scale(), state.message.clone(), 640.),
                actions(state),
                inputs::inputs(state),
                samples::samples(state),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::GROUPS.px()),
        )
        .padding(space::GROUPS.px()),
    )
    .constrain_horizontal(true);
    // The specimen sits on the same grained desk as Writer, so hover is judged against grain.
    desk(state.theme.grain.clone(), page).background_color(state.theme.palette.canvas)
}
fn header(state: &Specimen) -> impl WidgetView<Specimen> + use<> {
    flex_col((
        flex_row((label(state.scale(), "Recite · native component specimen")
            .typeset(Type::Display, state.scale())
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
        .gap(space::SIBLINGS.px()),
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
        .gap(space::SIBLINGS.px()),
        checkbox(
            "Reduced motion",
            state.config.reduced_motion,
            |state: &mut Specimen, checked| state.config.reduced_motion = checked,
        )
        .text_size(Type::Body.size(state.scale())),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
}
fn actions(state: &Specimen) -> impl WidgetView<Specimen> + use<> {
    flex_col((
        label(state.scale(), "Actions and navigation").typeset(Type::Section, state.scale()),
        flex_col((
            print::primary_action(
                state.scale(),
                "Try scene",
                &state.theme.palette,
                false,
                |state: &mut Specimen| state.message = "Primary action activated.".into(),
            ),
            text_button(state.scale(), "Add beat", |state: &mut Specimen| {
                state.message = "Secondary action activated.".into()
            }),
            print::action(
                state.scale(),
                "Arrange automatically",
                false,
                |state: &mut Specimen| state.message = "Quiet action activated.".into(),
            ),
            text_button(state.scale(), "Save changes", |_: &mut Specimen| {}).disabled(true),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
}
