//! Structural manuscript context never acquires source-mutating controls here.
use super::*;
pub(super) fn context(
    writer: &Writer,
    row: &beat::Context,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let index = row.index;
    flex_col((
        (!row.groups.is_empty()).then(|| {
            paragraph(
                writer.ui_scale(),
                format!("Within {}", row.groups.join(" → ")),
                width,
            )
        }),
        paragraph(
            writer.ui_scale(),
            row.pages.get(row.page).cloned().unwrap_or_default(),
            width,
        ),
        (row.pages.len() > 1).then(|| {
            flex_row((
                text_button(
                    writer.ui_scale(),
                    "Previous beat context part",
                    move |writer: &mut Writer| {
                        if let Some(row) = writer.localisation.context_row_mut(index) {
                            row.page = row.page.saturating_sub(1);
                        }
                    },
                )
                .disabled(row.page == 0),
                text_button(
                    writer.ui_scale(),
                    "More beat context",
                    move |writer: &mut Writer| {
                        if let Some(row) = writer.localisation.context_row_mut(index) {
                            row.page = (row.page + 1).min(row.pages.len().saturating_sub(1));
                        }
                    },
                )
                .disabled(row.page + 1 >= row.pages.len()),
            ))
            .gap(8.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(6.px())
}
