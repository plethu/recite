//! Beat identity and reading actions adapt without replacing the manuscript below.
use super::*;
use crate::WriterLayout;
use xilem::masonry::properties::LineBreaking;

pub(super) fn heading(state: &Writer, id: &str) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let compact = state.compact_chrome();
    let wide = state.script_width() >= 240. * f64::from(scale);
    let blocked = state.phase() != Phase::Writing || state.navigation.busy() || state.work.busy();
    flex_col((flex_row((
        print::title(scale, display_name(id))
            .text_size(if compact { 24. } else { 32. } * scale)
            .prop(LineBreaking::WordWrap)
            .flex(if wide { 1. } else { 0. }),
        print::action(scale, "Pin reference", blocked, Writer::pin_reference),
    ))
    .direction(if wide {
        Axis::Horizontal
    } else {
        Axis::Vertical
    })
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
    .width(state.script_width().px()),))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
