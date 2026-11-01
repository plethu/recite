use super::*;
use crate::attention::PaneEntry;
use xilem::masonry::kurbo::Axis;
pub(super) fn files(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let state = &writer.localisation;
    let queue = matches!(state.destination, Destination::Queue);
    flex_col((
        state.slot.path().filter(|_| !queue).map(|_| paragraph(scale, entry::catalogue_caption(writer), writer.content_width())),
        queue.then(|| flex_row((
            text_button(scale, if state.files_open { "Hide catalogue files" } else { "Catalogue files" }, |writer: &mut Writer| writer.localisation.files_open = !writer.localisation.files_open),
            state.has_catalogue().then(|| text_button(scale, "Translate current beat", |writer: &mut Writer| writer.open_translation_beat(0)).disabled(writer.project_action_blocked())),
            state.has_catalogue().then(|| text_button(scale, "Save all translation drafts", Writer::save_translation_drafts).disabled(writer.project_action_blocked() || state.slot.read_only() || !state.needs_save())),
        )).direction(if writer.viewport.width - writer.navigation_width() < 600. * f64::from(scale) { Axis::Vertical } else { Axis::Horizontal }).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px()).prop(PaneEntry(true))),
        (queue && (state.files_open || !state.has_catalogue())).then(|| flex_col((
            controls(writer),
            state.has_catalogue().then(|| text_button(scale, "Compare catalogue disk changes", Writer::compare_catalogue).disabled(writer.project_action_blocked() || state.slot.read_only())),
            text_button(scale, "Add language", Writer::new_catalogue).disabled(writer.project_action_blocked() || state.slot.read_only() || state.dirty()),
            state.has_catalogue().then(|| text_button(scale, "Review source updates", Writer::review_catalogue_sources).disabled(writer.project_action_blocked() || state.slot.read_only() || state.dirty())),
        )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())),
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
