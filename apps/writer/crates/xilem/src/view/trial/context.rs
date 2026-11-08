//! A short captured summary stays subordinate to prose; full provenance is explicit.
use super::*;
use crate::WriterLayout;
use recite_writer_model::PreviewProvenance;
use xilem::masonry::properties::LineBreaking;

pub(super) fn summary(
    state: &Writer,
    snapshot: &PreviewProvenance,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    flex_col((
        flex_row((
            print::meta(scale, snapshot.summary.clone())
                .text_size(0.7 * state.reading_size())
                .prop(LineBreaking::WordWrap)
                .color(state.palette.muted)
                .flex(1.),
            text_button(
                scale,
                if state.trial.context_details {
                    "Hide preview context"
                } else {
                    "Preview context"
                },
                |state: &mut Writer| state.trial.context_details = !state.trial.context_details,
            ),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(8.px())
        .width(state.script_width().px()),
        state.trial.context_details.then(|| {
            flex_row((print::meta(scale, snapshot.caption.clone())
                .text_size(0.7 * state.reading_size())
                .prop(LineBreaking::WordWrap)
                .color(state.palette.muted)
                .width(state.script_width().px()),))
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(4.px())
}
