use super::*;
use crate::{
    attention::{Pane, Region},
    presentation::{label, manuscript_text, paragraph, source_text, text_button},
};
use xilem::{
    WidgetView,
    core::one_of::Either,
    masonry::layout::AsUnit,
    style::Style,
    view::{CrossAxisAlignment, flex_col, flex_row, sized_box},
};

pub(in crate::workspaces) fn panel(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    flex_col((
        label(writer.ui_scale(), "Compare disk changes").text_size(28. * writer.ui_scale()),
        paragraph(writer.ui_scale(), "Review the versions before choosing a resolution. Each decision keeps a complete recovery copy. Save writes the accepted result.", 640.),
        report(writer),
        comparison(writer),
        pending(writer),
        actions(writer),
        merge(writer),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(12.px()).prop(Region(Some(Pane::External)))
}
fn actions(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocked = writer.project_action_blocked();
    let stale = !writer.external_comparison_current();
    let scale = writer.ui_scale();
    flex_col((
        text_button(scale, "Inspect current disk version", Writer::compare_disk_changes).disabled(blocked),
        flex_col((
            text_button(scale, "Keep Writer version", Writer::keep_writer_version).disabled(blocked || stale),
            paragraph(scale, "Preserves the accepted document, field draft and Undo history. Your next Save replaces the inspected disk version.", 640.),
            text_button(scale, "Use inspected disk version", Writer::use_inspected_disk).disabled(blocked || stale),
            paragraph(scale, "Archives the complete current session, then adopts the inspected disk version as one Undo step.", 640.),
            text_button(scale, "Export merge draft", Writer::export_merge_draft).disabled(blocked || stale),
            paragraph(scale, "Combine versions in your IDE, then import the reviewed merge below.", 640.),
        )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px()),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(12.px())
}
fn comparison(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let panel = &writer.workspaces.external;
    let inspection = panel.inspection.as_ref()?;
    let scale = writer.ui_scale();
    Some(flex_col((
        label(scale, inspection.path.file_name().unwrap_or_default().to_string_lossy().into_owned()),
        inspection.changes.get(panel.page).map(|change| {
            flex_col((
                label(scale, format!("Changed passage · {} · {} of {}", change.caption, panel.page + 1, inspection.changes.len())),
                paired_sources(writer, change),
            )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())
        }),
        inspection.changes.is_empty().then(|| paragraph(scale, "The accepted source matches the inspected disk version. Any unapplied field draft is shown separately.", 640.)),
        (inspection.changes.len() > 1).then(|| flex_row((
            text_button(scale, "Previous source comparison", |writer: &mut Writer| writer.workspaces.external.page = writer.workspaces.external.page.saturating_sub(1)).disabled(panel.page == 0),
            text_button(scale, "More source comparison", |writer: &mut Writer| writer.workspaces.external.page += 1).disabled(panel.page + 1 >= inspection.changes.len()),
        )).gap(8.px())),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(12.px()))
}
fn paired_sources(writer: &Writer, change: &ComparisonRow) -> impl WidgetView<Writer> + use<> {
    let available = (writer.viewport.width - 72.).max(240.);
    let wide = available >= 920. * f64::from(writer.ui_scale());
    let width = if wide {
        (available - 12.) / 2.
    } else {
        available
    };
    let before = source(
        writer,
        "Accepted Writer source",
        change.before.as_ref(),
        width,
    );
    let after = source(
        writer,
        "Inspected disk version",
        change.after.as_ref(),
        width,
    );
    if wide {
        Either::A(
            flex_row((before, after))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(12.px()),
        )
    } else {
        Either::B(
            flex_col((before, after))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(8.px()),
        )
    }
}
fn source(
    writer: &Writer,
    caption: &'static str,
    value: Option<&String>,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let value = value
        .cloned()
        .unwrap_or_else(|| "(No text in this part)".into());
    sized_box(
        flex_col((
            label(scale, caption),
            flex_row((sized_box(source_text(14. * scale, value)).width((width - 24.).px()),)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
    .width(width.px())
    .padding(12.px())
    .background_color(writer.palette.inset)
}
fn pending(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let panel = &writer.workspaces.external;
    let pages = panel.inspection.as_ref()?.pending.as_ref()?;
    let value = pages
        .get(panel.draft_page)
        .cloned()
        .unwrap_or_else(|| "(Empty draft)".into());
    let scale = writer.ui_scale();
    Some(
        sized_box(
            flex_col((
                label(
                    scale,
                    format!(
                        "Unapplied field draft · part {} of {}",
                        panel.draft_page + 1,
                        pages.len().max(1)
                    ),
                ),
                flex_row((sized_box(manuscript_text(16. * scale, value)).width(640.px()),)),
                (pages.len() > 1).then(|| {
                    flex_row((
                        text_button(scale, "Previous field draft part", |writer: &mut Writer| {
                            writer.workspaces.external.draft_page =
                                writer.workspaces.external.draft_page.saturating_sub(1)
                        })
                        .disabled(panel.draft_page == 0),
                        text_button(scale, "More field draft", |writer: &mut Writer| {
                            writer.workspaces.external.draft_page += 1
                        })
                        .disabled(panel.draft_page + 1 >= pages.len()),
                    ))
                    .gap(8.px())
                }),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px()),
        )
        .padding(12.px())
        .background_color(writer.palette.surface),
    )
}
fn merge(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocked = writer.project_action_blocked();
    let stale = !writer.external_comparison_current();
    let artifact = writer
        .workspaces
        .external
        .inspection
        .as_ref()
        .and_then(|inspection| inspection.artifact.as_ref());
    let scale = writer.ui_scale();
    flex_col((
        label(scale, "Merge in your editor"),
        paragraph(scale, "Export a separate source draft containing your pending prose. Edit it in your IDE, then import it here. Import validates the result against the inspected session and disk version; Save writes the accepted merge.", 640.),
        artifact.map(|artifact| paragraph(scale, format!("Merge draft: {}", artifact.source.display()), 640.)),
        text_button(scale, "Open merge draft in editor", Writer::open_merge_draft).disabled(blocked || writer.external.busy() || artifact.is_none()),
        text_button(scale, "Import reviewed merge", Writer::import_reviewed_merge).disabled(blocked || stale || artifact.is_none()),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())
}
fn report(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let panel = &writer.workspaces.external;
    let value = panel.report.get(panel.report_page)?;
    let scale = writer.ui_scale();
    Some(
        flex_col((
            paragraph(scale, value.clone(), 640.),
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
                .gap(8.px())
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
}
