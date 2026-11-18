use super::*;
use crate::presentation::print::section_heading;
use crate::presentation::space;
use crate::presentation::tokens::size;
pub fn panel(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    flex_col((
        text_button(
            scale,
            if writer.workspaces.declarations.source.preview_open {
                "Hide source draft preview"
            } else {
                "Show source draft preview"
            },
            |writer: &mut Writer| writer.workspaces.declarations.source.toggle_preview(),
        ),
        writer
            .workspaces
            .declarations
            .source
            .preview_open
            .then(|| content(writer)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
}
fn content(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let panel = &writer.workspaces.declarations.source;
    let page = panel.preview.get(panel.page)?;
    let scale = writer.ui_scale();
    Some(
        xilem::view::sized_box(
            flex_col((
                section_heading(
                    scale,
                    format!(
                        "Read-only TOML preview · part {} of {}",
                        panel.page + 1,
                        panel.preview.len()
                    ),
                ),
                flex_row((
                    text_button(scale, "Previous source preview", |writer: &mut Writer| {
                        writer.workspaces.declarations.source.page =
                            writer.workspaces.declarations.source.page.saturating_sub(1)
                    })
                    .disabled(panel.page == 0),
                    text_button(scale, "More source preview", |writer: &mut Writer| {
                        writer.workspaces.declarations.source.page += 1
                    })
                    .disabled(panel.page + 1 >= panel.preview.len()),
                )),
                paragraph(scale, page.clone(), size::MEASURE),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px()),
        )
        .padding(space::KINDS.px())
        .background_color(writer.palette.inset),
    )
}
