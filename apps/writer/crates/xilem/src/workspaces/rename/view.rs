use crate::presentation::space;
use crate::presentation::{Type, Typeset};
mod comparison;
use super::*;
use crate::session::RecoveryReason;
use crate::{
    attention::{Pane, PaneEntry, Region},
    presentation::{label, paragraph, print, text_button},
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
        crate::view::tool_head(
            writer,
            "Rename project beat",
            head_actions(writer, reviewing),
        )
        .prop(PaneEntry(reviewing)),
        paragraph(
            writer.ui_scale(),
            "Review references before staging. One project Undo step; Save all writes the files.",
            width,
        ),
        report(writer),
        history(writer),
        indexed_stack((proposal(writer), summary(writer))).active(usize::from(reviewing)),
        comparison(writer),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
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
                    .typeset(Type::Body, scale)
                    .disabled(blocked)
                    .prop(TextAreaLabel::new("Original beat name")),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(space::UNIT.px())
            .width(field_width.px()),
            flex_col((
                label(scale, "New beat name"),
                text_input(panel.name.clone(), Writer::change_rename_name)
                    .typeset(Type::Body, scale)
                    .disabled(blocked)
                    .prop(TextAreaLabel::new("New beat name")),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(space::UNIT.px())
            .width(field_width.px()),
        ))
        .direction(if wide {
            Axis::Horizontal
        } else {
            Axis::Vertical
        })
        .gap(space::KINDS.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
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
    .gap(space::KINDS.px())
}
/// The sheet's own actions: Review is its primary while proposing; once reviewed, Stage rename is,
/// beside a tonal way back to the proposal.
fn head_actions(writer: &Writer, reviewing: bool) -> impl WidgetView<Writer> + use<> {
    let panel = &writer.workspaces.rename;
    let blocked = writer.project_action_blocked();
    let scale = writer.ui_scale();
    let review = writer
        .session
        .project()
        .is_some_and(|project| project.files().rename_review.is_some());
    let incomplete = panel.block.trim().is_empty()
        || panel.name.trim().is_empty()
        || panel.block.trim() == panel.name.trim();
    flex_row((
        reviewing.then(|| {
            text_button(scale, "Edit rename proposal", Writer::discard_rename_review)
                .disabled(blocked)
        }),
        print::primary_action(
            scale,
            if reviewing {
                "Stage rename"
            } else {
                "Review project rename"
            },
            &writer.palette,
            blocked || if reviewing { !review } else { incomplete },
            move |writer: &mut Writer| {
                if reviewing {
                    writer.apply_project_rename();
                } else {
                    writer.review_project_rename();
                }
            },
        ),
    ))
    .gap(space::SIBLINGS.px())
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
    .gap(space::SIBLINGS.px())
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
        .gap(space::SIBLINGS.px()),
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
        .gap(space::SIBLINGS.px()),
    )
}
