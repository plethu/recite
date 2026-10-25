//! Recovery stays beside the exact rejected draft; details do not enlarge fixed chrome.
use super::*;
use crate::presentation::group::status;

pub(super) fn panel(state: &Writer, width: f64) -> Option<impl WidgetView<Writer> + use<>> {
    let refusal = state.reading_refusal()?;
    let request = refusal.request();
    let scale = state.ui_scale();
    Some(status(
        refusal.title(),
        flex_col((
            label(scale, refusal.title()).color(state.palette.accent),
            paragraph(
                scale,
                "Your text is kept. Review the reason, then retry.",
                width,
            ),
            flex_row((
                text_button(scale, "Edit text", Writer::request_editing),
                text_button(
                    scale,
                    if refusal.expanded() {
                        "Hide reason"
                    } else {
                        "Show reason"
                    },
                    move |state: &mut Writer| state.explain_reading_refusal(&request),
                ),
            ))
            .direction(if width >= 260. * f64::from(scale) {
                Axis::Horizontal
            } else {
                Axis::Vertical
            })
            .gap(8.px()),
            refusal
                .reason()
                .map(|reason| paragraph(scale, reason.to_owned(), width)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(6.px())
        .width(width.px()),
    ))
}
