use super::*;
pub(super) fn files(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let state = &writer.localisation;
    let queue = matches!(state.destination, Destination::Queue);
    flex_col((
        state.slot.path().map(|path| {
            let path = writer.session.project().and_then(|project| path.strip_prefix(project.root()).ok()).unwrap_or(path);
            paragraph(scale, path.display().to_string(), 640.)
        }),
        state.slot.document().and_then(|document| document.headers().iter().find(|header| header.key().eq_ignore_ascii_case("Language"))).map(|header| label(scale, format!("Language: {}", header.value().chars().take(80).collect::<String>()))),
        (queue && state.has_catalogue()).then(|| text_button(scale, if state.files_open { "Hide catalogue files" } else { "Catalogue files" }, |writer: &mut Writer| writer.localisation.files_open = !writer.localisation.files_open)),
        (queue && (state.files_open || !state.has_catalogue())).then(|| controls(writer)),
        (queue && state.has_catalogue()).then(|| text_button(scale, "Save all translation drafts", Writer::save_translation_drafts).disabled(writer.project_action_blocked() || state.slot.read_only() || !state.needs_save())),
        (queue && state.has_catalogue()).then(|| text_button(scale, "Compare catalogue disk changes", Writer::compare_catalogue).disabled(writer.project_action_blocked() || state.slot.read_only())),
        state.slot.read_only().then(|| paragraph(scale, "The catalogue is read-only until its operation returns. After an unexpected stop, close with recovery and reopen before editing translations.", 640.)),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())
}
fn controls(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let blocked = writer.project_action_blocked() || writer.localisation.slot.read_only();
    flex_col((
        writer
            .localisation
            .slot
            .path()
            .map(|path| paragraph(scale, path.display().to_string(), 640.)),
        label(scale, "Catalogue path"),
        text_input(
            writer.localisation.path.clone(),
            |writer: &mut Writer, path| writer.localisation.path = path,
        )
        .text_size(16. * scale)
        .disabled(blocked)
        .prop(TextAreaLabel::new("Catalogue path")),
        flex_col((
            text_button(scale, "Open catalogue", Writer::open_catalogue_path).disabled(
                blocked
                    || writer.localisation.path.trim().is_empty()
                    || writer.localisation.dirty(),
            ),
            text_button(scale, "Reload catalogue", |writer: &mut Writer| {
                writer.catalogue_action(work::Action::Reload)
            })
            .disabled(
                blocked || !writer.localisation.has_catalogue() || writer.localisation.dirty(),
            ),
            text_button(scale, "Close catalogue", |writer: &mut Writer| {
                writer.catalogue_action(work::Action::Close)
            })
            .disabled(
                blocked || !writer.localisation.has_catalogue() || writer.localisation.dirty(),
            ),
        ))
        .gap(8.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
