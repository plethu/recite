//! Production manuscript/source fonts and the authored surface palette.
use super::*;
use crate::presentation::space;
use crate::presentation::tokens::size;
use crate::presentation::{Type, Typeset};
use crate::presentation::{manuscript_text, source_text};

pub(super) fn samples(state: &Specimen) -> impl WidgetView<Specimen> + use<> {
    let scale = state.scale();
    flex_col((
        label(scale, "Manuscript and source").typeset(Type::Section, scale),
        paragraph(
            scale,
            "Reading: 20 px · Source: 15 px. These text sizes are independent of UI scale.",
            size::MEASURE,
        ),
        flex_row((manuscript_text(20., state.manuscript.clone()).width(size::MEASURE.px()),)),
        flex_row((source_text(
            15.,
            ":: harbour default\n> @narrator\n  The last tram waits.\n-> END",
        )
        .width(size::MEASURE.px()),)),
        label(scale, "Surfaces and selection").typeset(Type::Section, scale),
        surface(state, "Reading surface", state.theme.palette.surface),
        surface(state, "Inset control surface", state.theme.palette.inset),
        surface(state, "Selected passage", state.theme.palette.selection),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
}
fn surface(
    state: &Specimen,
    caption: &'static str,
    color: xilem::peniko::Color,
) -> impl WidgetView<Specimen> + use<> {
    label(state.scale(), caption)
        .padding(space::KINDS.px())
        .width(size::MEASURE.px())
        .background_color(color)
}
