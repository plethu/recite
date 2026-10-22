use super::*;
use crate::attention::PaneEntry;
use crate::navigation::Target;
use recite_writer_session::location::Screen as LinkedScreen;
mod forms;
pub(super) use forms::catalogue_caption;
mod plural;
mod source;
#[cfg(test)]
mod tests;
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
            navigation(writer, blocked),
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

fn navigation(writer: &Writer, blocked: bool) -> impl WidgetView<Writer> + use<> {
    let state = &writer.localisation;
    let scale = writer.ui_scale();
    flex_col((
        state.passage.as_ref().map(|beat| {
            let name = beat.name.clone();
            let page = beat.page;
            text_button(
                scale,
                "Return to bilingual beat",
                move |writer: &mut Writer| {
                    let mut location = writer.current_location();
                    location.screen = LinkedScreen::Localise;
                    location.beat = Some(name.clone());
                    location.passage = None;
                    location.page = page;
                    writer.navigate(Target::Location(location));
                },
            )
            .disabled(blocked)
        }),
        text_button(
            scale,
            "Return to translation queue",
            |writer: &mut Writer| writer.catalogue_action(work::Action::Query),
        )
        .disabled(writer.project_action_blocked()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
    .prop(PaneEntry(true))
}
