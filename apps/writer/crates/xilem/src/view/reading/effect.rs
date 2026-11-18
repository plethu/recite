//! An effect's row: the writer's reading of its call, with the source line it was written as beside
//! it in the source labels' face.
use super::*;
use xilem::masonry::properties::LineBreaking;

/// `reading` in the meta face, then `source` in the id face on the rest of the line, clipped where
/// it would not fit; both muted, as notes about the script rather than its words.
pub(super) fn effect(
    state: &Writer,
    (reading, source): (&str, &str),
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    flex_row((
        print::meta(scale, reading.to_owned()).color(state.palette.muted),
        print::id(scale, source.to_owned())
            .color(state.palette.muted)
            .prop(LineBreaking::Clip)
            .flex(1.),
    ))
    .cross_axis_alignment(CrossAxisAlignment::FirstBaseline)
    .gap(space::SIBLINGS.px())
    .width(width.px())
}
