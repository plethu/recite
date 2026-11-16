use super::*;
use crate::presentation::space;
use crate::presentation::tokens::{breakpoint, size};
use crate::{
    attention::{Pane, Region},
    presentation::{Type, label, manuscript_text, paragraph, print, text_button},
};
use xilem::{
    WidgetView,
    core::one_of::Either,
    masonry::layout::AsUnit,
    style::Style,
    view::{CrossAxisAlignment, flex_col, flex_row, sized_box},
};

pub(in crate::workspaces) fn panel(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    if let Some(resolved) = writer.workspaces.external.resolved() {
        return Either::A(resolved::panel(writer, resolved));
    }
    Either::B(
        flex_col((
            crate::view::tool_head(writer, "Compare disk changes", resolution(writer)),
            paragraph(
                writer.ui_scale(),
                "Each choice keeps a complete recovery copy; Save writes the accepted result.",
                size::MEASURE,
            ),
            report(writer),
            comparison(writer),
            pending(writer),
            merge(writer),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::TEXT.px())
        .prop(Region(Some(Pane::External))),
    )
}
mod comparison;
mod resolved;
use comparison::{comparison, pending};
/// The resolution in the sheet's head: Keep mine, or the primary, Use disk version. A comparison
/// the disk has moved past offers to inspect the current disk version instead.
fn resolution(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocked = writer.project_action_blocked();
    let stale = !writer.external_comparison_current();
    let scale = writer.ui_scale();
    flex_row((
        stale.then(|| {
            text_button(
                scale,
                "Inspect current disk version",
                Writer::compare_disk_changes,
            )
            .disabled(blocked)
        }),
        text_button(scale, "Keep mine", Writer::keep_writer_version).disabled(blocked || stale),
        print::primary_action(
            scale,
            "Use disk version",
            &writer.palette,
            blocked || stale,
            Writer::use_inspected_disk,
        ),
    ))
    .gap(space::SIBLINGS.px())
}
/// Merging in the editor as one quiet row: export a merge draft, then open or import it.
fn merge(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocked = writer.project_action_blocked();
    let stale = !writer.external_comparison_current();
    let artifact = writer
        .workspaces
        .external
        .inspection()
        .and_then(|inspection| inspection.artifact.as_ref());
    let scale = writer.ui_scale();
    let exported = artifact.is_some();
    let actions = (
        (!exported).then(|| {
            text_button(scale, "Export merge draft", Writer::export_merge_draft)
                .disabled(blocked || stale)
        }),
        exported.then(|| {
            text_button(
                scale,
                "Open merge draft in editor",
                Writer::open_merge_draft,
            )
            .disabled(blocked || writer.external.busy())
        }),
        exported.then(|| {
            text_button(
                scale,
                "Import reviewed merge",
                Writer::import_reviewed_merge,
            )
            .disabled(blocked || stale)
        }),
    );
    flex_row((
        Type::Control.label(scale, "Or merge in your editor"),
        flex_row(actions).gap(space::SIBLINGS.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(space::TEXT.px())
}
/// The comparison's report, every part of it.
fn report(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let report = &writer.workspaces.external.report;
    (!report.is_empty()).then(|| {
        let width = writer.content_width().min(size::MEASURE);
        flex_col(
            report
                .iter()
                .map(|part| paragraph(writer.ui_scale(), part.clone(), width))
                .collect::<Vec<_>>(),
        )
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px())
    })
}
