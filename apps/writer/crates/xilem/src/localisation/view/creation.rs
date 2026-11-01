use super::*;
use recite_writer_session::localisation::{create, setup::languages};

pub(super) fn creation(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    if !matches!(writer.localisation.destination, Destination::Create) {
        return None;
    }
    let scale = writer.ui_scale();
    let state = &writer.localisation.creation;
    let blocked = writer.project_action_blocked() || writer.localisation.slot.read_only();
    let path = writer
        .session
        .project_root()
        .zip(create::language(&state.selected).ok())
        .map(|(root, locale)| create::suggested_path(root, &locale.to_string()));
    Some(flex_col((
        label(scale, "Add a language").text_size(20. * scale),
        paragraph(scale, "Creates a new PO catalogue from this project's current sources. Apply manuscript fields first. Accepted unsaved source edits are included; source files stay unchanged.", 640.),
        label(scale, "Language name or tag"),
        text_input(state.query.clone(), |writer: &mut Writer, value| writer.localisation.creation.query = value).text_size(16. * scale).disabled(blocked).prop(TextAreaLabel::new("Language name or tag")),
        text_button(scale, "Search languages", Writer::search_catalogue_languages).disabled(blocked),
        state.selected.is_empty().then(|| choices(writer)),
        (!state.query.trim().is_empty() && state.choices.is_empty()).then(|| paragraph(scale, "Search by language name, native name or a tag such as fr-CA.", 640.)),
        languages::caption(&state.selected).map(|caption| paragraph(scale, format!("Selected language: {caption}"), 640.)),
        (!state.selected.is_empty()).then(|| text_button(scale, "Choose another language", |writer: &mut Writer| writer.localisation.creation.selected.clear()).disabled(blocked)),
        path.as_ref().map(|path| {
            let display = writer.session.project_root().and_then(|root| path.strip_prefix(root).ok()).unwrap_or(path);
            paragraph(scale, format!("New catalogue: {}", display.display()), 640.)
        }),
        text_button(scale, "Create catalogue", Writer::create_catalogue).disabled(blocked || path.is_none()),
        text_button(scale, "Return to translation queue", Writer::return_to_translation_queue).disabled(blocked),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(10.px()))
}
fn choices(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let state = &writer.localisation.creation;
    let scale = writer.ui_scale();
    let choices = state
        .choices
        .iter()
        .skip(state.page * 8)
        .take(8)
        .map(|choice| {
            let value = choice.value.clone();
            text_button(
                scale,
                format!("Choose {} · {}", choice.title, choice.value),
                move |writer: &mut Writer| writer.localisation.creation.selected = value.clone(),
            )
            .disabled(writer.project_action_blocked())
        })
        .collect::<Vec<_>>();
    flex_col((
        flex_col(choices)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(6.px()),
        (state.choices.len() > 8).then(|| {
            flex_col((
                label(
                    scale,
                    format!(
                        "Languages {}–{} of {}",
                        state.page * 8 + 1,
                        ((state.page + 1) * 8).min(state.choices.len()),
                        state.choices.len()
                    ),
                ),
                text_button(scale, "Previous languages", |writer: &mut Writer| {
                    writer.localisation.creation.page =
                        writer.localisation.creation.page.saturating_sub(1)
                })
                .disabled(state.page == 0 || writer.project_action_blocked()),
                text_button(scale, "More languages", |writer: &mut Writer| {
                    writer.localisation.creation.page += 1
                })
                .disabled(
                    (state.page + 1) * 8 >= state.choices.len() || writer.project_action_blocked(),
                ),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(6.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
