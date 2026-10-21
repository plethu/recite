use super::*;
use crate::presentation::manuscript_text;
mod forms;
mod plural;
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
            label(scale, "Source passage").text_size(20. * scale),
            entry.source.get(entry.source_page).map(|source| {
                flex_row((
                    sized_box(manuscript_text(writer.reading_size(), source.clone()))
                        .width(writer.script_width().px()),
                ))
            }),
            (entry.source.len() > 1).then(|| {
                flex_row((
                    text_button(
                        scale,
                        "Previous translation source part",
                        |writer: &mut Writer| {
                            if let Some(entry) = &mut writer.localisation.entry {
                                entry.source_page = entry.source_page.saturating_sub(1);
                            }
                        },
                    )
                    .disabled(entry.source_page == 0),
                    text_button(scale, "More translation source", |writer: &mut Writer| {
                        if let Some(entry) = &mut writer.localisation.entry {
                            entry.source_page += 1;
                        }
                    })
                    .disabled(entry.source_page + 1 >= entry.source.len()),
                ))
                .gap(8.px())
            }),
            entry
                .context
                .get(entry.context_page)
                .map(|context| paragraph(scale, format!("Nearby source: {context}"), 640.)),
            (entry.context.len() > 1).then(|| {
                flex_col((
                    label(
                        scale,
                        format!(
                            "Source context part {} of {}",
                            entry.context_page + 1,
                            entry.context.len()
                        ),
                    ),
                    text_button(scale, "Previous source context", |writer: &mut Writer| {
                        if let Some(entry) = &mut writer.localisation.entry {
                            entry.context_page = entry.context_page.saturating_sub(1);
                        }
                    })
                    .disabled(entry.context_page == 0),
                    text_button(scale, "More source context", |writer: &mut Writer| {
                        if let Some(entry) = &mut writer.localisation.entry {
                            entry.context_page += 1;
                        }
                    })
                    .disabled(entry.context_page + 1 >= entry.context.len()),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(6.px())
            }),
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
