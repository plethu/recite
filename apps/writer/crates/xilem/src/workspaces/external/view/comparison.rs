//! The inspected change set: each changed passage's two versions side by side where the sheet is
//! wide, and the unapplied field draft that neither version holds.
use super::*;

/// An unapplied field draft is shown in its manuscript face at this size, before the UI scale.
const DRAFT_TEXT: f32 = 16.;

pub(super) fn comparison(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
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
pub(super) fn pending(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
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
