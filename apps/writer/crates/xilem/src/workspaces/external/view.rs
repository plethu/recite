use super::*;
use crate::presentation::space;
use crate::presentation::tokens::{breakpoint, size};
use crate::{
    attention::{Pane, Region},
    presentation::{
        Type, label, manuscript_text, paragraph, print, source_comparison, text_button,
    },
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
                "Review the versions before choosing a resolution. Each \
                 decision keeps a complete recovery copy. Save writes the \
                 accepted result.",
                size::MEASURE,
            ),
            report(writer),
            comparison(writer),
            pending(writer),
            actions(writer),
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
/// The resolution choice in the sheet's head: inspect, keep, or the primary, adopt the inspected
/// disk version.
fn resolution(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocked = writer.project_action_blocked();
    let stale = !writer.external_comparison_current();
    let scale = writer.ui_scale();
    flex_row((
        text_button(
            scale,
            "Inspect current disk version",
            Writer::compare_disk_changes,
        )
        .disabled(blocked),
        text_button(scale, "Keep Writer version", Writer::keep_writer_version)
            .disabled(blocked || stale),
        print::primary_action(
            scale,
            "Use inspected disk version",
            &writer.palette,
            blocked || stale,
            Writer::use_inspected_disk,
        ),
    ))
    .gap(space::SIBLINGS.px())
}
fn actions(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocked = writer.project_action_blocked();
    let stale = !writer.external_comparison_current();
    let scale = writer.ui_scale();
    flex_col((
        paragraph(
            scale,
            "Keep Writer version preserves the accepted document, field \
             draft and Undo history; your next Save replaces the \
             inspected disk version. Use inspected disk version archives \
             the complete current session, then adopts the disk version \
             as one Undo step.",
            size::MEASURE,
        ),
        text_button(scale, "Export merge draft", Writer::export_merge_draft)
            .disabled(blocked || stale),
        paragraph(
            scale,
            "Combine versions in your IDE, then import the reviewed merge below.",
            size::MEASURE,
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
}
fn merge(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocked = writer.project_action_blocked();
    let stale = !writer.external_comparison_current();
    let artifact = writer
        .workspaces
        .external
        .inspection()
        .and_then(|inspection| inspection.artifact.as_ref());
    let scale = writer.ui_scale();
    flex_col((
        label(scale, "Merge in your editor"),
        paragraph(
            scale,
            "Export a separate source draft containing your pending \
             prose. Edit it in your IDE, then import it here. Import \
             validates the result against the inspected session and disk \
             version; Save writes the accepted merge.",
            size::MEASURE,
        ),
        artifact.map(|artifact| {
            paragraph(
                scale,
                format!("Merge draft: {}", artifact.source.display()),
                size::MEASURE,
            )
        }),
        text_button(
            scale,
            "Open merge draft in editor",
            Writer::open_merge_draft,
        )
        .disabled(blocked || writer.external.busy() || artifact.is_none()),
        text_button(
            scale,
            "Import reviewed merge",
            Writer::import_reviewed_merge,
        )
        .disabled(blocked || stale || artifact.is_none()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
}
fn report(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let panel = &writer.workspaces.external;
    let value = panel.report.get(panel.report_page)?;
    let scale = writer.ui_scale();
    Some(
        flex_col((
            paragraph(
                scale,
                value.clone(),
                writer.content_width().min(size::MEASURE),
            ),
            (panel.report.len() > 1).then(|| {
                flex_row((
                    text_button(
                        scale,
                        "Previous comparison report",
                        |writer: &mut Writer| {
                            writer.workspaces.external.report_page =
                                writer.workspaces.external.report_page.saturating_sub(1)
                        },
                    )
                    .disabled(panel.report_page == 0),
                    text_button(scale, "More comparison report", |writer: &mut Writer| {
                        writer.workspaces.external.report_page += 1
                    })
                    .disabled(panel.report_page + 1 >= panel.report.len()),
                ))
                .gap(space::SIBLINGS.px())
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px()),
    )
}
