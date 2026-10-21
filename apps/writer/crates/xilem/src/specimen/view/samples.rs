//! Production manuscript/source fonts and the authored surface palette.
use super::*;
use crate::presentation::{manuscript_text, source_text};

pub(super) fn samples(state: &Specimen) -> impl WidgetView<Specimen> + use<> {
    let scale = state.scale();
    flex_col((
        label(scale, "Manuscript and source").text_size(22. * scale),
        paragraph(
            scale,
            "Reading: 20 px · Source: 15 px. These text sizes are independent of UI scale.",
            640.,
        ),
        flex_row((manuscript_text(20., state.manuscript.clone()).width(640.px()),)),
        flex_row((source_text(
            15.,
            ":: harbour default\n> narrator@7701ceab59d2adfa057a\n  The last tram waits.\n-> END",
        )
        .width(640.px()),)),
        label(scale, "Surfaces and selection").text_size(22. * scale),
        surface(state, "Reading surface", state.theme.palette.surface),
        surface(state, "Inset control surface", state.theme.palette.inset),
        surface(state, "Selected passage", state.theme.palette.selection),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}
fn surface(
    state: &Specimen,
    caption: &'static str,
    color: xilem::peniko::Color,
) -> impl WidgetView<Specimen> + use<> {
    label(state.scale(), caption)
        .padding(12.px())
        .width(640.px())
        .background_color(color)
}
