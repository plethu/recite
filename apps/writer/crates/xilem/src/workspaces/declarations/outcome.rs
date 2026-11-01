//! Bounded declaration action feedback is shared by source and producer screens.
use super::*;
pub(super) fn panel(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let page = writer
        .workspaces
        .schema_feedback
        .get(writer.workspaces.schema_feedback_page)?;
    let scale = writer.ui_scale();
    Some(
        flex_col((
            label(scale, "Declaration action result").text_size(18. * scale),
            paragraph(scale, page.clone(), 640.),
            flex_row((
                text_button(scale, "Previous schema result", |writer: &mut Writer| {
                    writer.workspaces.schema_feedback_page =
                        writer.workspaces.schema_feedback_page.saturating_sub(1)
                })
                .disabled(writer.workspaces.schema_feedback_page == 0),
                text_button(scale, "More schema result", |writer: &mut Writer| {
                    writer.workspaces.schema_feedback_page += 1
                })
                .disabled(
                    writer.workspaces.schema_feedback_page + 1
                        >= writer.workspaces.schema_feedback.len(),
                ),
            )),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start),
    )
}
