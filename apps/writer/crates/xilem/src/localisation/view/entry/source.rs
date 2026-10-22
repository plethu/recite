//! Source prose and navigation are one composed section of a translation entry.
use super::*;
use crate::{localisation::source_navigation::SourceAction, presentation::manuscript_text};

pub(super) fn source(
    writer: &Writer,
    entry: &Entry,
    blocked: bool,
) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let show_key = entry.key.clone();
    let preview_key = entry.key.clone();
    let blocked = blocked
        || writer.project_action_blocked()
        || writer.localisation.saving()
        || entry.source_location.is_none();
    flex_col((
        label(scale, "Source passage").text_size(20. * scale),
        entry.source_location.as_ref().map(|caption| {
            paragraph(
                scale,
                format!("Source location: {caption}"),
                writer.script_width(),
            )
        }),
        entry.source.get(entry.source_page).map(|source| {
            flex_row((
                sized_box(manuscript_text(prose_size(writer), source.clone()))
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
        text_button(scale, "Show source passage", move |writer: &mut Writer| {
            writer.open_translation_source(&show_key, SourceAction::Show)
        })
        .disabled(blocked),
        text_button(scale, "Preview this passage", move |writer: &mut Writer| {
            writer.open_translation_source(&preview_key, SourceAction::Preview)
        })
        .disabled(blocked),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(10.px())
}
