//! A source passage and its translation actions compose independently of the queue.
use super::*;
use crate::presentation::manuscript_text;

pub(super) fn passage(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let context = writer.localisation.passage.as_ref()?;
    Some(
        flex_col((
            label(writer.ui_scale(), "Selected source passage").text_size(22. * writer.ui_scale()),
            source(writer, context),
            translation_actions(writer),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(12.px()),
    )
}
fn source(writer: &Writer, context: &location::PassageContext) -> impl WidgetView<Writer> + use<> {
    let value = context.pages.get(context.page).cloned().unwrap_or_default();
    flex_col((
        flex_row((sized_box(manuscript_text(writer.reading_size(), value)).width(640.px()),)),
        context
            .pages
            .is_empty()
            .then(|| paragraph(writer.ui_scale(), "This source passage is empty.", 640.)),
        source_pages(writer, context),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
fn source_pages(
    writer: &Writer,
    context: &location::PassageContext,
) -> Option<impl WidgetView<Writer> + use<>> {
    if context.pages.len() <= 1 {
        return None;
    }
    Some(
        flex_col((
            label(
                writer.ui_scale(),
                format!(
                    "Source part {} of {}",
                    context.page + 1,
                    context.pages.len()
                ),
            ),
            (context.page + 1 < context.pages.len()).then(|| {
                paragraph(
                    writer.ui_scale(),
                    "This passage continues in the next source part.",
                    640.,
                )
            }),
            text_button(
                writer.ui_scale(),
                "Previous source part",
                |writer: &mut Writer| {
                    if let Some(context) = &mut writer.localisation.passage {
                        context.page = context.page.saturating_sub(1);
                    }
                },
            )
            .disabled(context.page == 0),
            text_button(
                writer.ui_scale(),
                "More source passage",
                |writer: &mut Writer| {
                    if let Some(context) = &mut writer.localisation.passage {
                        context.page += 1;
                    }
                },
            )
            .disabled(context.page + 1 >= context.pages.len()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
}
fn translation_actions(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let state = &writer.localisation;
    let translate = state.entry.as_ref().map(|_| {
        text_button(scale, "Translate this passage", |writer: &mut Writer| {
            writer.localisation.destination = Destination::Entry;
        })
    });
    let unmatched = (state.entry.is_none() && state.has_catalogue()).then(|| {
        paragraph(scale, "This source passage has no matching translation entry. Review source updates to add it to the catalogue.", 640.)
    });
    let queue = text_button(scale, "Open translation queue", |writer: &mut Writer| {
        writer.localisation.destination = Destination::Queue;
        writer.localisation.files_open = !writer.localisation.has_catalogue();
    });
    flex_col((translate, unmatched, queue))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(12.px())
}
