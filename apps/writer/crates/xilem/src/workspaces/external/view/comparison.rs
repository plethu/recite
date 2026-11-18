//! The inspected change set: every changed passage as a row, its two versions side by side where
//! the sheet is wide, and the unapplied field draft that neither version holds. The sheet scrolls;
//! nothing pages.
use super::*;
use crate::WriterLayout;
use crate::presentation::diff::{Side, Text, side_by_side};
use xilem::masonry::parley::style::FontWeight;

/// An unapplied field draft is shown in its manuscript face at this size, before the UI scale.
const DRAFT_TEXT: f32 = 16.;

pub(super) fn comparison(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let inspection = writer.workspaces.external.inspection()?;
    let scale = writer.ui_scale();
    let width = writer.page_width();
    let rows = inspection
        .changes
        .iter()
        .map(|change| {
            flex_col((
                Type::Control
                    .label(scale, format!("Changed passage · {}", change.caption))
                    .weight(FontWeight::SEMI_BOLD),
                side_by_side(
                    &writer.palette,
                    scale,
                    (width, breakpoint::COMPARISON * f64::from(scale)),
                    (
                        Side {
                            caption: "Writer version",
                            text: change.before.as_deref(),
                        },
                        Side {
                            caption: "Disk version",
                            text: change.after.as_deref(),
                        },
                    ),
                    Text::Source(Type::Body.size(scale)),
                ),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px())
        })
        .collect::<Vec<_>>();
    Some(
        flex_col((
            print::id(
                scale,
                inspection
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
            )
            .color(writer.palette.muted),
            inspection.changes.is_empty().then(|| {
                paragraph(
                    scale,
                    "The accepted source matches the inspected disk version. Any \
                     unapplied field draft is shown separately.",
                    size::MEASURE,
                )
            }),
            flex_col(rows)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(space::KINDS.px()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::TEXT.px()),
    )
}
/// The unapplied field draft, all of it, in its manuscript face.
pub(super) fn pending(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let pages = writer.workspaces.external.inspection()?.pending.as_ref()?;
    let value = if pages.is_empty() {
        "(Empty draft)".to_owned()
    } else {
        pages.concat()
    };
    let scale = writer.ui_scale();
    Some(
        sized_box(
            flex_col((
                label(scale, "Unapplied field draft"),
                flex_row(
                    (sized_box(manuscript_text(DRAFT_TEXT * scale, value)).width(
                        (writer.page_width() - space::GROUPS)
                            .min(size::MEASURE)
                            .px(),
                    ),),
                ),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px()),
        )
        .padding(space::KINDS.px())
        .background_color(writer.palette.inset),
    )
}
