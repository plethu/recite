//! A beat's continuation set on the manuscript grid, editable or read-only, at one place.
use super::*;

/// How a destination is set: as a route button, whose own padding brings its words in, or as a
/// read-only label padded by the same inset so both read at one place.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::view::reading) enum RouteFace {
    Button,
    Label,
}
impl RouteFace {
    fn inset(self, scale: f32) -> f64 {
        match self {
            Self::Button => 0.,
            Self::Label => control_inset(scale),
        }
    }
}

/// A beat's continuation on the grid: `margin` in the margin column, and `route` in the
/// destination column where there is one, or under the text column where destinations wrap.
pub(in crate::view::reading) fn continuation_row<M: WidgetView<Writer>, R: WidgetView<Writer>>(
    state: &Writer,
    grid: &Grid,
    margin: Option<M>,
    route: R,
    face: RouteFace,
) -> impl WidgetView<Writer> + use<M, R> {
    let scale = state.ui_scale();
    let column = grid.destination > 0.;
    let pad = if column {
        // Like a reply's route: the button's padding takes the rest of the gutter.
        (DESTINATION_GUTTER - GUTTER - control_inset(scale)).max(0.) + face.inset(scale)
    } else {
        face.inset(scale)
    };
    flex_row((
        flex_row((FlexSpacer::Flex(1.), margin)).width(grid.margin.px()),
        column.then(|| flex_col(()).width(grid.measure.px())),
        sized_box(route).padding(Padding::left(pad.px())),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(grid.gutter().px())
}

/// The continuation set on row `index`'s own line: the beat's jump straight after a line of
/// dialogue, outside any group. A jump elsewhere then takes no row of its own; the end of the
/// conversation keeps its end mark beneath.
pub(in crate::view::reading) fn joined(rows: &[Row], index: usize) -> Option<&str> {
    let [line, next] = rows.get(index..index + 2)? else {
        return None;
    };
    match (&line.content, &next.content) {
        (Content::Passage { passage, .. }, Content::Continuation(target))
            if line.groups.is_empty()
                && next.groups.is_empty()
                && matches!(passage.kind, PassageKind::Dialogue { .. }) =>
        {
            Some(target)
        }
        _ => None,
    }
}

/// Whether row `index` is a jump elsewhere set on the line before it.
pub(in crate::view::reading) fn absorbed(rows: &[Row], index: usize) -> bool {
    index > 0 && joined(rows, index - 1).is_some_and(|target| target != "END")
}
