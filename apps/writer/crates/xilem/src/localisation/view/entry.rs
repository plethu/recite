use super::*;
use crate::WriterLayout;
use crate::attention::PaneEntry;
use crate::navigation::Target;
use recite_writer_session::localisation::catalogue::Draft;
use recite_writer_session::location::Screen as LinkedScreen;
use xilem::view::{GridExt, GridTrackSize, grid};
mod forms;
mod plural;
mod source;
#[cfg(test)]
mod tests;
mod wordings;
pub(super) fn entry(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let state = &writer.localisation;
    let entry = state.entry.as_ref()?;
    let draft = state.slot.draft(entry.key.id)?;
    let blocked = state.slot.read_only() || writer.phase() != Phase::Writing;
    Some(
        flex_col((
            navigation(writer, blocked),
            grid((
                source::source(writer, entry, blocked).grid_pos(0, u16::from(!wide(writer))),
                editor(writer, entry, &draft, blocked).grid_pos(u16::from(wide(writer)), 0),
            ))
            .columns(if wide(writer) {
                vec![GridTrackSize::Fixed(entry_width(writer).px()); 2]
            } else {
                vec![GridTrackSize::Fixed(entry_width(writer).px())]
            })
            .rows(vec![GridTrackSize::Auto; if wide(writer) { 1 } else { 2 }])
            .gap(24.px()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(10.px()),
    )
}

fn wide(writer: &Writer) -> bool {
    writer.content_width() >= 800. * f64::from(writer.ui_scale())
}
fn entry_width(writer: &Writer) -> f64 {
    writer.script_width().min(if wide(writer) {
        (writer.content_width() - 24.) / 2.
    } else {
        writer.content_width()
    })
}
fn editor(
    writer: &Writer,
    entry: &Entry,
    draft: &Draft,
    blocked: bool,
) -> impl WidgetView<Writer> + use<> {
    let state = &writer.localisation;
    let scale = writer.ui_scale();
    let reviewed_key = entry.key.clone();
    let discard_key = entry.key.clone();
    let changed = state
        .slot
        .owner()
        .is_some_and(|owner| owner.catalogue.changed(entry.key.id));
    flex_col((
        wordings::wordings(writer, entry, blocked),
        plural::guide(writer, entry, blocked),
        forms::forms(writer, entry, draft, blocked),
        flex_row((
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
        ))
        .direction(if entry_width(writer) < 320. * f64::from(scale) {
            Axis::Vertical
        } else {
            Axis::Horizontal
        })
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
        text_button(
            scale,
            "Discard translation draft",
            move |writer: &mut Writer| writer.discard_translation(&discard_key),
        )
        .disabled(blocked || state.saving() || !changed),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(10.px())
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
