//! A translation change's two versions, drawn as every Writer diff is: side by side on inset paper
//! where the sheet is wide, stacked where it is not.
use super::*;
use crate::WriterLayout;
use crate::presentation::diff::{Side, Text, side_by_side};
pub(super) fn versions(
    writer: &Writer,
    row: &ComparisonRow,
    before_caption: &'static str,
    after_caption: &'static str,
) -> impl WidgetView<Writer> + use<> {
    side_by_side(
        &writer.palette,
        writer.ui_scale(),
        (writer.page_width(), 920. * f64::from(writer.ui_scale())),
        (
            Side {
                caption: before_caption,
                text: row.before.as_deref(),
            },
            Side {
                caption: after_caption,
                text: row.after.as_deref(),
            },
        ),
        Text::Manuscript(prose_size(writer)),
    )
}
