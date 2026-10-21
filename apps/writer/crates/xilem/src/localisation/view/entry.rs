use super::*;
mod forms;
mod plural;
mod source;
mod wordings;
pub(super) fn entry(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let state = &writer.localisation;
    let entry = state.entry.as_ref()?;
    let draft = state.slot.draft(entry.key.id)?;
    let scale = writer.ui_scale();
    let blocked = state.slot.read_only() || writer.phase != Phase::Writing;
    let reviewed_key = entry.key.clone();
    let discard_key = entry.key.clone();
    let changed = state
        .slot
        .owner()
        .is_some_and(|owner| owner.catalogue.changed(entry.key.id));
    Some(
        flex_col((
            text_button(
                scale,
                "Return to translation queue",
                |writer: &mut Writer| writer.catalogue_action(work::Action::Query),
            )
            .disabled(writer.project_action_blocked()),
            source::source(writer, entry, blocked),
            wordings::wordings(writer, entry, blocked),
            plural::guide(writer, entry, blocked),
            forms::forms(writer, entry, &draft, blocked),
            checkbox(
                "Reviewed translation",
                draft.reviewed,
                move |writer: &mut Writer, reviewed| {
                    writer.review_translation(&reviewed_key, reviewed)
                },
            )
            .text_size(14. * scale)
            .disabled(blocked),
            text_button(scale, "Save translation", Writer::save_translation_entry).disabled(
                blocked || writer.project_jobs_busy() || !changed || !entry.details.plural.valid(),
            ),
            text_button(
                scale,
                "Discard translation draft",
                move |writer: &mut Writer| writer.discard_translation(&discard_key),
            )
            .disabled(blocked || state.saving() || !changed),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(10.px()),
    )
}
