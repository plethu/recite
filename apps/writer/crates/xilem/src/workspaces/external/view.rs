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
mod resolved;

/// An unapplied field draft is shown in its manuscript face at this size, before the UI scale.
const DRAFT_TEXT: f32 = 16.;
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
fn comparison(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let panel = &writer.workspaces.external;
    let inspection = panel.inspection()?;
    let scale = writer.ui_scale();
    Some(
        flex_col((
            label(
                scale,
                inspection
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
            ),
            inspection.changes.get(panel.page).map(|change| {
                flex_col((
                    label(
                        scale,
                        format!(
                            "Changed passage · {} · {} of {}",
                            change.caption,
                            panel.page + 1,
                            inspection.changes.len()
                        ),
                    ),
                    paired_sources(writer, change),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(space::SIBLINGS.px())
            }),
            inspection.changes.is_empty().then(|| {
                paragraph(
                    scale,
                    "The accepted source matches the inspected disk version. Any \
                     unapplied field draft is shown separately.",
                    size::MEASURE,
                )
            }),
            (inspection.changes.len() > 1).then(|| {
                flex_row((
                    text_button(
                        scale,
                        "Previous source comparison",
                        |writer: &mut Writer| {
                            writer.workspaces.external.page =
                                writer.workspaces.external.page.saturating_sub(1)
                        },
                    )
                    .disabled(panel.page == 0),
                    text_button(scale, "More source comparison", |writer: &mut Writer| {
                        writer.workspaces.external.page += 1
                    })
                    .disabled(panel.page + 1 >= inspection.changes.len()),
                ))
                .gap(space::SIBLINGS.px())
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::TEXT.px()),
    )
}
fn paired_sources(writer: &Writer, change: &ComparisonRow) -> impl WidgetView<Writer> + use<> {
    let available = writer.content_width();
    let wide = available >= breakpoint::COMPARISON * f64::from(writer.ui_scale());
    let width = if wide {
        (available - space::TEXT) / 2.
    } else {
        available
    };
    let before = source(
        writer,
        "Accepted Writer source",
        change.before.as_ref(),
        change.after.as_ref(),
        width,
    );
    let after = source(
        writer,
        "Inspected disk version",
        change.after.as_ref(),
        change.before.as_ref(),
        width,
    );
    if wide {
        Either::A(
            flex_row((before, after))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(space::TEXT.px()),
        )
    } else {
        Either::B(
            flex_col((before, after))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(space::SIBLINGS.px()),
        )
    }
}
fn source(
    writer: &Writer,
    caption: &'static str,
    value: Option<&String>,
    other: Option<&String>,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let value = value
        .cloned()
        .unwrap_or_else(|| "(No text in this part)".into());
    sized_box(
        flex_col((
            label(scale, caption),
            flex_row((sized_box(source_comparison(
                Type::Body.size(scale),
                value,
                other.cloned().unwrap_or_default(),
            ))
            .width((width - space::GROUPS).px()),)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px()),
    )
    .width(width.px())
    .padding(space::KINDS.px())
    .background_color(writer.palette.inset)
}
fn pending(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let panel = &writer.workspaces.external;
    let pages = panel.inspection()?.pending.as_ref()?;
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
                flex_row(
                    (sized_box(manuscript_text(DRAFT_TEXT * scale, value)).width(
                        (writer.content_width() - space::GROUPS)
                            .min(size::MEASURE)
                            .px(),
                    ),),
                ),
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
                    .gap(space::SIBLINGS.px())
                }),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px()),
        )
        .padding(space::KINDS.px())
        .background_color(writer.palette.surface),
    )
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
