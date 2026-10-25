//! One stationary navigation row for the bounded, overlapping manuscript window.
use super::*;
use crate::reading::Direction;

pub(in crate::view) fn page_controls(
    state: &Writer,
    surface: &Surface<'_>,
) -> Option<impl WidgetView<Writer> + use<>> {
    let total = state.reading.rows.len();
    let window = state.reading.window;
    let range = window.range(total);
    (range.len() < total && matches!(surface, Surface::Manuscript { .. } | Surface::Beat { .. }))
        .then(|| {
            let blocked =
                state.phase != Phase::Writing || state.navigation.busy() || state.work.busy();
            flex_row((
                print::icon_action(
                    state.ui_scale(),
                    "Previous beat entries",
                    Icon::Back,
                    &state.palette,
                    blocked || window.shifted(Direction::Previous, total).is_none(),
                    |state: &mut Writer| state.shift_reading(Direction::Previous),
                ),
                print::meta(
                    state.ui_scale(),
                    format!("Entries {}–{} of {total}", range.start + 1, range.end),
                ),
                print::icon_action(
                    state.ui_scale(),
                    "Next beat entries",
                    Icon::Forward,
                    &state.palette,
                    blocked || window.shifted(Direction::Next, total).is_none(),
                    |state: &mut Writer| state.shift_reading(Direction::Next),
                ),
                print::action(
                    state.ui_scale(),
                    "Find in this beat",
                    blocked,
                    |state: &mut Writer| {
                        state.app_request = Some(Request::Search(Search::Passages))
                    },
                ),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap(12.px())
        })
}
