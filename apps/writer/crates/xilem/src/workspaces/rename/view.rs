use crate::WriterLayout;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::space;
use crate::presentation::tokens::size;
use crate::presentation::{Type, Typeset};
mod comparison;
#[cfg(test)]
mod tests;
use super::*;
use crate::presentation::marks::{GLYPH, wedge_glyph};
use crate::session::RecoveryReason;
use crate::{
    attention::{Pane, PaneEntry, Region},
    presentation::{label, paragraph, print, text_button},
};
use comparison::comparison;
use recite_writer_app::workspaces::rename::Report;
use xilem::{
    WidgetView,
    masonry::{layout::AsUnit, properties::TextAreaLabel},
    style::Style,
    view::{CrossAxisAlignment, flex_col, flex_row, indexed_stack, text_input},
};

pub(in crate::workspaces) fn panel(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let width = writer.page_width();
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
/// The fallback after a refused rename, or a rename opened from a link: the beat as a label and
/// value beside the field holding its new id, the name already typed. A clash the review would
/// refuse is said under the field in the error ink after the error sign, and marks it invalid
/// with an error border; what Review still waits for is said under it and describes Review.
fn proposal(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let panel = &writer.workspaces.rename;
    let blocked = writer.project_action_blocked();
    let scale = writer.ui_scale();
    let width = writer
        .page_width()
        .min(size::FIELD_COLUMN * f64::from(scale));
    let clash = writer.rename_clash();
    flex_col((
        flex_row((
            print::meta(scale, "Beat").color(writer.palette.muted),
            print::id(
                scale,
                if panel.block.is_empty() {
                    "None chosen".to_owned()
                } else {
                    panel.block.clone()
                },
            ),
        ))
        .cross_axis_alignment(CrossAxisAlignment::FirstBaseline)
        .gap(space::SIBLINGS.px()),
        label(scale, "New beat name"),
        styled(
            if clash.is_some() {
                ControlStyle::InvalidField
            } else {
                ControlStyle::Field
            },
            text_input(panel.name.clone(), Writer::change_rename_name)
                .typeset(Type::Body, scale)
                .disabled(blocked || panel.block.is_empty())
                .prop(TextAreaLabel::new("New beat name").with_error(clash.clone())),
        ),
        // The refusal in the error ink after the error sign, read as an error.
        clash.map(|clash| {
            flex_row((
                wedge_glyph(GLYPH * f64::from(scale), writer.palette.error),
                print::caption(scale, format!("Error: {clash}")).color(writer.palette.error),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap(space::UNIT.px())
        }),
        // What Review still waits for, under the field it concerns; Review reads it too.
        reason(writer).map(|reason| print::caption(scale, reason).color(writer.palette.muted)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(space::UNIT.px())
    .width(width.px())
    .prop(PaneEntry(true))
}
/// What a proposal still lacks before it can be reviewed, while it is proposed.
fn reason(writer: &Writer) -> Option<&'static str> {
    let panel = &writer.workspaces.rename;
    panel
        .changes
        .is_empty()
        .then(|| missing(&panel.block, &panel.name, writer.rename_clash().is_some()))
        .flatten()
}
/// What a proposal still lacks before it can be reviewed, said under its field.
fn missing(block: &str, name: &str, clash: bool) -> Option<&'static str> {
    if clash {
        Some("Choose an id no other beat in this file has")
    } else if block.trim().is_empty() {
        Some("Choose Rename beat in Commands to pick a beat")
    } else if name.trim().is_empty() {
        Some("Enter a new beat id")
    } else if block.trim() == name.trim() {
        Some("Enter an id different from the beat's own")
    } else {
        None
    }
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
    let blocked = writer.project_action_blocked();
    let scale = writer.ui_scale();
    let review = writer
        .session
        .project()
        .is_some_and(|project| project.files().rename_review.is_some());
    let missing = (!reviewing).then(|| reason(writer)).flatten();
    flex_row((
        reviewing.then(|| {
            text_button(scale, "Cancel", Writer::discard_rename_review)
                .accessible_label("Cancel rename")
                .disabled(blocked)
        }),
        print::primary_described(
            scale,
            if reviewing {
                "Rename"
            } else {
                "Review project rename"
            },
            &writer.palette,
            (
                blocked
                    || if reviewing {
                        !review
                    } else {
                        missing.is_some()
                    },
                false,
            ),
            missing,
            move |writer: &mut Writer| {
                if reviewing {
                    writer.apply_project_rename();
                } else {
                    writer.review_project_rename();
                }
            },
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
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
                writer.page_width().min(size::MEASURE),
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
    let report = writer.workspaces.rename.report.as_ref()?;
    // A refusal the field already explains in the writer's words is not repeated in the
    // compiler's.
    if matches!(report, Report::Problem(_)) && writer.rename_clash().is_some() {
        return None;
    }
    let pages = report.pages();
    let width = writer.page_width().min(size::MEASURE);
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
