use super::*;
use crate::presentation::source_comparison;

pub(super) fn comparison(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let panel = &writer.workspaces.rename;
    let change = panel.changes.get(panel.page)?;
    let scale = writer.ui_scale();
    Some(
        flex_col((
            label(
                scale,
                format!(
                    "Review · {} · {} of {}",
                    change.caption,
                    panel.page + 1,
                    panel.changes.len()
                ),
            ),
            flex_row((
                text_button(scale, "Previous rename change", |writer: &mut Writer| {
                    writer.workspaces.rename.page = writer.workspaces.rename.page.saturating_sub(1)
                })
                .disabled(panel.page == 0),
                text_button(scale, "More rename changes", |writer: &mut Writer| {
                    writer.workspaces.rename.page += 1
                })
                .disabled(panel.page + 1 >= panel.changes.len()),
            )),
            change.before.as_ref().map(|text| {
                sized_box(
                    flex_col((
                        label(scale, "Before"),
                        flex_row((sized_box(source_comparison(
                            14. * scale,
                            text.clone(),
                            change.after.clone().unwrap_or_default(),
                        ))
                        .width((writer.content_width() - 24.).min(960.).px()),)),
                    ))
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .gap(8.px()),
                )
                .padding(12.px())
                .background_color(writer.palette.inset)
            }),
            change.after.as_ref().map(|text| {
                sized_box(
                    flex_col((
                        label(scale, "After"),
                        flex_row((sized_box(source_comparison(
                            14. * scale,
                            text.clone(),
                            change.before.clone().unwrap_or_default(),
                        ))
                        .width((writer.content_width() - 24.).min(960.).px()),)),
                    ))
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .gap(8.px()),
                )
                .padding(12.px())
                .background_color(writer.palette.surface)
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(12.px()),
    )
}
