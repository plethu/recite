mod comparison;
use super::*;
use crate::session::RecoveryReason;
use crate::{
    attention::{Pane, PaneEntry, Region},
    presentation::{label, paragraph, text_button},
};
use comparison::comparison;
use xilem::{
    WidgetView,
    masonry::{kurbo::Axis, layout::AsUnit, properties::TextAreaLabel},
    style::Style,
    view::{CrossAxisAlignment, flex_col, flex_row, indexed_stack, sized_box, text_input},
};

pub(in crate::workspaces) fn panel(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let width = writer.content_width();
    let reviewing = !writer.workspaces.rename.changes.is_empty();
    flex_col((
        label(writer.ui_scale(), "Rename project beat").text_size(24. * writer.ui_scale()),
        paragraph(
            writer.ui_scale(),
            "Review references before staging. One project Undo step; Save all writes the files.",
            width,
        ),
        report(writer),
        history(writer),
        indexed_stack((proposal(writer), summary(writer))).active(usize::from(reviewing)),
        review_action(writer, reviewing),
        actions(writer),
        comparison(writer),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
    .prop(Region(Some(Pane::Rename)))
}
fn proposal(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let panel = &writer.workspaces.rename;
    let blocked = writer.project_action_blocked();
    let scale = writer.ui_scale();
    let wide = writer.content_width() >= 520. * f64::from(scale);
    let field_width = if wide {
        (writer.content_width() - 16.) / 2.
    } else {
        writer.content_width()
    };
    flex_col((
        text_button(scale, "Choose existing beat", Writer::choose_rename_beat).disabled(blocked),
        flex_row((
            flex_col((
                label(scale, "Original beat name"),
                text_input(panel.block.clone(), Writer::change_rename_block)
                    .text_size(16. * scale)
                    .disabled(blocked)
                    .prop(TextAreaLabel::new("Original beat name")),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(6.px())
            .width(field_width.px()),
            flex_col((
                label(scale, "New beat name"),
                text_input(panel.name.clone(), Writer::change_rename_name)
                    .text_size(16. * scale)
                    .disabled(blocked)
                    .prop(TextAreaLabel::new("New beat name")),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(6.px())
            .width(field_width.px()),
        ))
        .direction(if wide {
            Axis::Horizontal
        } else {
            Axis::Vertical
        })
        .gap(16.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
    .prop(PaneEntry(true))
}
fn summary(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let panel = &writer.workspaces.rename;
    let wide = writer.content_width() >= 520. * f64::from(writer.ui_scale());
    let measure = if wide {
        writer.content_width() - 160. * f64::from(writer.ui_scale()) - 16.
    } else {
        writer.content_width()
    };
    flex_row((
        paragraph(
            writer.ui_scale(),
            format!("{} → {}", panel.block, panel.name),
            measure,
        ),
        text_button(
            writer.ui_scale(),
            "Choose existing beat",
            Writer::choose_rename_beat,
        )
        .disabled(writer.project_action_blocked()),
    ))
    .direction(if wide {
        Axis::Horizontal
    } else {
        Axis::Vertical
    })
    .gap(16.px())
}
fn review_action(writer: &Writer, reviewing: bool) -> impl WidgetView<Writer> + use<> {
    let panel = &writer.workspaces.rename;
    text_button(
        writer.ui_scale(),
        if reviewing {
            "Edit rename proposal"
        } else {
            "Review project rename"
        },
        move |writer: &mut Writer| {
            if reviewing {
                writer.discard_rename_review();
            } else {
                writer.review_project_rename();
            }
        },
    )
    .disabled(
        writer.project_action_blocked()
            || (!reviewing
                && (panel.block.trim().is_empty()
                    || panel.name.trim().is_empty()
                    || panel.block.trim() == panel.name.trim())),
    )
}
fn actions(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let blocked = writer.project_action_blocked();
    let review = writer
        .session
        .project()
        .is_some_and(|project| project.files().rename_review.is_some());
    (!writer.workspaces.rename.changes.is_empty()).then(|| {
        flex_row((text_button(
            writer.ui_scale(),
            "Apply project rename",
            Writer::apply_project_rename,
        )
        .disabled(blocked || !review),))
        .prop(PaneEntry(true))
    })
}
fn history(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocked = writer.project_action_blocked();
    let scale = writer.ui_scale();
    flex_row((
        writer.has_project_history(false).then(|| {
            text_button(scale, "Undo project rename", Writer::undo_project_rename).disabled(blocked)
        }),
        writer.has_project_history(true).then(|| {
            text_button(scale, "Redo project rename", Writer::redo_project_rename).disabled(blocked)
        }),
    ))
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
            paragraph(writer.ui_scale(), message, writer.content_width().min(640.)),
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
fn report(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let panel = &writer.workspaces.rename;
    let pages = panel.report.as_ref()?.pages();
    let text = pages.get(panel.report_page)?;
    Some(
        flex_col((
            paragraph(
                writer.ui_scale(),
                text.clone(),
                writer.content_width().min(640.),
            ),
            (pages.len() > 1).then(|| {
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
                    .disabled(panel.report_page + 1 >= pages.len()),
                ))
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
}
