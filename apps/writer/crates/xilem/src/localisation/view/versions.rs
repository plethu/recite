use super::*;
use crate::presentation::manuscript_text;
use xilem::core::one_of::Either;
pub(super) fn versions(
    writer: &Writer,
    row: &ComparisonRow,
    before_caption: &'static str,
    after_caption: &'static str,
) -> impl WidgetView<Writer> + use<> {
    let available = (writer.viewport.width - 72.).max(240.);
    let wide = available >= 920. * f64::from(writer.ui_scale());
    let width = if wide {
        (available - 12.) / 2.
    } else {
        available
    };
    let before = version(writer, before_caption, row.before.as_deref(), width);
    let after = version(writer, after_caption, row.after.as_deref(), width);
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
                .gap(12.px()),
        )
    }
}
fn version(
    writer: &Writer,
    caption: &'static str,
    value: Option<&str>,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    sized_box(
        flex_col((
            label(writer.ui_scale(), caption),
            flex_row((sized_box(manuscript_text(
                writer.reading_size(),
                value.unwrap_or("(No text in this part)").to_owned(),
            ))
            .width((width - 24.).px()),)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
    .width(width.px())
    .padding(12.px())
    .background_color(writer.palette.inset)
}
