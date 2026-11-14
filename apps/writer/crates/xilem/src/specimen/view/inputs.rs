//! Native text, radio and checkbox controls retain their ordinary keyboard behavior.
use super::*;
use crate::presentation::segments::{segment, segments};
use xilem::{InsertNewline, masonry::properties::TextAreaLabel, view::text_input};

pub(super) fn inputs(state: &Specimen) -> impl WidgetView<Specimen> + use<> {
    let scale = state.scale();
    let palette = &state.theme.palette;
    let choices = [Workspace::Script, Workspace::Map, Workspace::Split].map(|workspace| {
        let caption = match workspace {
            Workspace::Script => "Script",
            Workspace::Map => "Map",
            Workspace::Split => "Split",
        };
        segment(
            scale,
            palette,
            caption,
            state.workspace == workspace,
            false,
            move |state: &mut Specimen| state.workspace = workspace,
        )
    });
    let destinations = ["Missing Courier", "Station History", "Goodbye"]
        .into_iter()
        .filter(|title| title.to_lowercase().contains(&state.query.to_lowercase()))
        .map(|title| {
            text_button(scale, title, move |state: &mut Specimen| {
                state.destination = title.into();
                state.message = format!("Destination selected: {title}.");
            })
        })
        .collect::<Vec<_>>();
    flex_col((
        label(scale, "Fields and selection").text_size(22. * scale),
        segments(palette, choices),
        label(scale, "Find a destination"),
        flex_row((
            text_input(state.query.clone(), |state: &mut Specimen, text| {
                state.query = text
            })
            .placeholder("Search scene titles…")
            .text_size(16. * scale)
            .prop(TextAreaLabel::new("Find a destination"))
            .width(640.px()),
        )),
        label(scale, format!("Destination: {}", state.destination)),
        flex_col(destinations)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px()),
        label(scale, "Manuscript"),
        sized_box(
            text_input(state.manuscript.clone(), |state: &mut Specimen, text| {
                state.manuscript = text
            })
            .insert_newline(InsertNewline::OnEnter)
            .font("Literata")
            .text_size(20.)
            .prop(TextAreaLabel::new("Specimen manuscript")),
        )
        .dims((640.px(), 180.px())),
        checkbox(
            "Reviewed translation",
            state.reviewed,
            |state: &mut Specimen, checked| state.reviewed = checked,
        )
        .text_size(14. * scale),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}
