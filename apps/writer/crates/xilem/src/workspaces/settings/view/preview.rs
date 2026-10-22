use super::*;
use crate::presentation::source_text;

pub(super) fn preview(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let panel = &writer.workspaces.settings;
    let scale = writer.ui_scale();
    flex_col((
        text_button(
            scale,
            if panel.preview_open {
                "Hide settings preview"
            } else {
                "Show settings preview"
            },
            |writer: &mut Writer| writer.workspaces.settings.toggle_preview(),
        ),
        panel
            .preview
            .get(panel.page)
            .filter(|_| panel.preview_open)
            .map(|page| {
                sized_box(
                    flex_col((
                        writer
                            .project_settings()
                            .and_then(Session::reviewed_path)
                            .map(|path| {
                                paragraph(
                                    scale,
                                    format!("Reviewed from: {}", path.display()),
                                    writer.content_width() - 24.,
                                )
                            }),
                        label(
                            scale,
                            format!(
                                "Read-only TOML preview · part {} of {}",
                                panel.page + 1,
                                panel.preview.len()
                            ),
                        ),
                        flex_row((
                            text_button(
                                scale,
                                "Previous settings preview",
                                |writer: &mut Writer| {
                                    writer.workspaces.settings.page =
                                        writer.workspaces.settings.page.saturating_sub(1)
                                },
                            )
                            .disabled(panel.page == 0),
                            text_button(scale, "More settings preview", |writer: &mut Writer| {
                                writer.workspaces.settings.page += 1
                            })
                            .disabled(panel.page + 1 >= panel.preview.len()),
                        )),
                        flex_row((sized_box(source_text(14. * scale, page.clone()))
                            .width((writer.content_width() - 24.).min(960.).px()),)),
                    ))
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .gap(8.px()),
                )
                .padding(12.px())
                .background_color(writer.palette.inset)
            }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
