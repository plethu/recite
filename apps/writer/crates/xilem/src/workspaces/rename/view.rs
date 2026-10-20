use super::*;
use crate::session::RecoveryReason;
use crate::{
    attention::{Pane, Region},
    presentation::{label, paragraph, text_button},
};
use xilem::{
    WidgetView,
    masonry::{layout::AsUnit, properties::TextAreaLabel},
    style::Style,
    view::{CrossAxisAlignment, flex_col, flex_row, sized_box, text_input},
};

pub(in crate::workspaces) fn panel(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    flex_col((
        label(writer.ui_scale(),"Rename project beat").text_size(28. * writer.ui_scale()),
        paragraph(writer.ui_scale(),"Review references across the project before staging a rename. Staged changes share one Undo/Redo step. Save all writes the project files.",640.),
        proposal(writer),
        actions(writer),
        report(writer),
        comparison(writer),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(12.px()).prop(Region(Some(Pane::Rename)))
}
fn proposal(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let panel = &writer.workspaces.rename;
    let blocked = writer.project_action_blocked();
    let scale = writer.ui_scale();
    flex_col((
        label(scale, "Current beat name"),
        text_input(panel.block.clone(), Writer::change_rename_block)
            .text_size(16. * scale)
            .disabled(blocked)
            .prop(TextAreaLabel::new("Current beat name")),
        label(scale, "New beat name"),
        text_input(panel.name.clone(), Writer::change_rename_name)
            .text_size(16. * scale)
            .disabled(blocked)
            .prop(TextAreaLabel::new("New beat name")),
        text_button(
            scale,
            "Review project rename",
            Writer::review_project_rename,
        )
        .disabled(
            blocked
                || panel.block.trim().is_empty()
                || panel.name.trim().is_empty()
                || panel.block.trim() == panel.name.trim(),
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
fn actions(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocked = writer.project_action_blocked();
    let review = writer
        .session
        .project()
        .is_some_and(|project| project.files().rename_review.is_some());
    let scale = writer.ui_scale();
    flex_col((
        flex_row((
            text_button(scale, "Apply project rename", Writer::apply_project_rename)
                .disabled(blocked || !review),
            text_button(
                scale,
                "Discard rename review",
                Writer::discard_rename_review,
            )
            .disabled(blocked || !review),
        ))
        .gap(8.px()),
        flex_row((
            text_button(scale, "Undo project rename", Writer::undo_project_rename)
                .disabled(blocked || !writer.has_project_history(false)),
            text_button(scale, "Redo project rename", Writer::redo_project_rename)
                .disabled(blocked || !writer.has_project_history(true)),
        ))
        .gap(8.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
pub(crate) fn recovery(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let reason = writer.session.recovery_reason()?;
    let message = match reason {
        RecoveryReason::RetryCheckpoint => {
            "Editing is paused to protect grouped recovery. Retry recovery, or Close and keep recovery to reopen safely."
        }
        RecoveryReason::ReopenRequired => {
            "The project operation stopped unexpectedly. Close and keep recovery, then reopen the project before editing."
        }
    };
    Some(
        flex_col((
            paragraph(writer.ui_scale(), message, 640.),
            (reason == RecoveryReason::RetryCheckpoint).then(|| {
                text_button(
                    writer.ui_scale(),
                    "Retry project edit recovery",
                    Writer::finish_project_recovery,
                )
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
}
fn comparison(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
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
                    flex_col((label(scale, "Before"), paragraph(scale, text.clone(), 640.)))
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .gap(8.px()),
                )
                .padding(12.px())
                .background_color(writer.palette.inset)
            }),
            change.after.as_ref().map(|text| {
                sized_box(
                    flex_col((label(scale, "After"), paragraph(scale, text.clone(), 640.)))
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
fn report(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let panel = &writer.workspaces.rename;
    let text = panel.report.get(panel.report_page)?;
    Some(
        flex_col((
            paragraph(writer.ui_scale(), text.clone(), 640.),
            (panel.report.len() > 1).then(|| {
                flex_row((
                    text_button(
                        writer.ui_scale(),
                        "Previous rename report",
                        |writer: &mut Writer| {
                            writer.workspaces.rename.report_page =
                                writer.workspaces.rename.report_page.saturating_sub(1)
                        },
                    )
                    .disabled(panel.report_page == 0),
                    text_button(
                        writer.ui_scale(),
                        "More rename report",
                        |writer: &mut Writer| writer.workspaces.rename.report_page += 1,
                    )
                    .disabled(panel.report_page + 1 >= panel.report.len()),
                ))
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
}
