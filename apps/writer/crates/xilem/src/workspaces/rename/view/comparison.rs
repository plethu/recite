//! A reviewed rename's references as rows: where each one is (file, beat and line) over its
//! change, drawn side by side. The list scrolls with the sheet; nothing pages.
use super::*;
use crate::WriterLayout;
use crate::presentation::diff::{Side, Text, side_by_side};
use crate::presentation::print::section_heading;
use crate::presentation::space;
use xilem::masonry::parley::style::FontWeight;

pub(super) fn comparison(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let panel = &writer.workspaces.rename;
    if panel.changes.is_empty() {
        return None;
    }
    let scale = writer.ui_scale();
    let width = writer.page_width().min(960. * f64::from(scale));
    let rows = panel
        .changes
        .iter()
        .map(|change| {
            flex_col((
                Type::Control
                    .label(scale, change.caption.clone())
                    .weight(FontWeight::SEMI_BOLD),
                side_by_side(
                    &writer.palette,
                    scale,
                    (width, 640. * f64::from(scale)),
                    (
                        Side {
                            caption: "Before",
                            text: change.before.as_deref(),
                        },
                        Side {
                            caption: "After",
                            text: change.after.as_deref(),
                        },
                    ),
                    Text::Source(14. * scale),
                ),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px())
        })
        .collect::<Vec<_>>();
    Some(
        flex_col((
            section_heading(
                scale,
                format!(
                    "{} {}",
                    panel.changes.len(),
                    if panel.changes.len() == 1 {
                        "changed reference"
                    } else {
                        "changed references"
                    }
                ),
            ),
            flex_col(rows)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(space::KINDS.px()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::TEXT.px()),
    )
}
