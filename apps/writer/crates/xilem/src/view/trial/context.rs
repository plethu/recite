//! A short captured summary stays subordinate to prose; full provenance is explicit.
use super::*;
use crate::WriterLayout;
use crate::presentation::space;
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
        .gap(space::SIBLINGS.px())
        .width(state.script_width().px()),
        state.trial.context_details.then(|| {
            flex_row((print::meta(scale, snapshot.caption.clone())
                .prop(LineBreaking::WordWrap)
                .color(state.palette.muted)
                .width(state.script_width().px()),))
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::UNIT.px())
}
