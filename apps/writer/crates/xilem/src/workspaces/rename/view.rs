use crate::presentation::space;
use crate::presentation::tokens::size;
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
    view::{CrossAxisAlignment, flex_col, flex_row, indexed_stack, text_input},
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
/// While reviewing, the proposal reads as one line: the beat and its new id.
fn summary(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let panel = &writer.workspaces.rename;
    let scale = writer.ui_scale();
    print::id(scale, format!("{} → {}", panel.block, panel.name))
}
/// The sheet's own actions: Review is its primary while proposing; once reviewed, Rename is,
/// beside a tonal Cancel back to the proposal.
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
            text_button(scale, "Cancel", Writer::discard_rename_review)
                .accessible_label("Cancel rename")
                .disabled(blocked)
        }),
        print::primary_action(
            scale,
            if reviewing {
                "Rename"
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
            paragraph(
                writer.ui_scale(),
                message,
                writer.content_width().min(size::MEASURE),
            ),
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
/// The last rename's report, every part of it.
fn report(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let pages = writer.workspaces.rename.report.as_ref()?.pages();
    let width = writer.content_width().min(size::MEASURE);
    Some(
        flex_col(
            pages
                .iter()
                .map(|text| paragraph(writer.ui_scale(), text.clone(), width))
                .collect::<Vec<_>>(),
        )
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px()),
    )
}
