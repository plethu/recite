use super::*;
use crate::attention::PaneEntry;

pub(super) fn refresh(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let state = &writer.localisation;
    let refresh = state.refresh.as_ref()?;
    let scale = writer.ui_scale();
    let blocked = writer.project_action_blocked() || state.slot.read_only();
    let width = writer.content_width();
    let current = state
        .slot
        .owner()
        .is_some_and(|owner| refresh.preview.is_current(&owner.catalogue));
    Some(flex_col((
        label(scale, "Source updates").text_size(20. * scale),
        paragraph(scale, refresh.preview.summary.clone(), width),
        flex_row((
            text_button(scale, "Recheck source updates", Writer::review_catalogue_sources).disabled(blocked || state.dirty()),
            text_button(scale, "Apply source updates to PO", Writer::apply_catalogue_sources).disabled(blocked || !current),
            text_button(scale, "Return to translation queue", Writer::return_to_translation_queue).disabled(blocked),
        )).direction(if width >= 580. * f64::from(scale) { Axis::Horizontal } else { Axis::Vertical })
            .cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px()).prop(PaneEntry(true)),
        paragraph(scale, "Compares accepted project sources, including unsaved edits. Apply rechecks the review, preserves translations and notes, and writes only the PO file.", width),
        refresh.pages.get(refresh.page).map(|page| flex_col((
            paragraph(scale, format!("{} · part {} of {}", page.source.caption, refresh.page + 1, refresh.pages.len()), 640.),
            versions::versions(writer, &page.source, "Previous source", "Current project source"),
            page.detail.as_ref().map(|detail| paragraph(scale, detail.clone(), 640.)),
        )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())),
        refresh.pages.is_empty().then(|| paragraph(scale, "No source text changes. Applying also refreshes source metadata.", 640.)),
        paging(writer),
        (!current).then(|| paragraph(scale, "The catalogue changed after this review. Recheck before applying source updates.", 640.)),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(12.px()))
}
fn paging(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let refresh = writer.localisation.refresh.as_ref()?;
    (refresh.pages.len() > 1).then(|| {
        flex_col((
            text_button(
                writer.ui_scale(),
                "Previous source update part",
                |writer: &mut Writer| {
                    if let Some(refresh) = &mut writer.localisation.refresh {
                        refresh.page = refresh.page.saturating_sub(1);
                    }
                },
            )
            .disabled(refresh.page == 0),
            text_button(
                writer.ui_scale(),
                "More source updates",
                |writer: &mut Writer| {
                    if let Some(refresh) = &mut writer.localisation.refresh {
                        refresh.page += 1;
                    }
                },
            )
            .disabled(refresh.page + 1 >= refresh.pages.len()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px())
    })
}
