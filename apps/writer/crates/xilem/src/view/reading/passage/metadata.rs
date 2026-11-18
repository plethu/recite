//! What sits beside or under a reply's text, its destination button, and the margin's source label:
//! the stable id of the row being written.
use super::*;
use crate::presentation::sheet_text::one_line;
use crate::presentation::{Type, hang_below};

/// Space between a margin label and the id under it.
const ID_GAP: f64 = 2.;

/// The margin column's `content` with, while the row is written, its stable id under it: set like
/// the reply numbers, right-aligned against the gutter and ending in an ellipsis where the column
/// is too narrow, its whole id still read aloud. Beside the measure the id hangs into the spacing
/// under the row, so choosing a row never moves another; above it, it takes its own line.
pub(super) fn with_id<V: WidgetView<Writer>>(
    state: &Writer,
    grid: &Grid,
    (passage, active): (&Passage, bool),
    content: V,
) -> impl WidgetView<Writer> + use<V> {
    let wide = grid.margin > 0.;
    let size = Sheet::Number.size(Page::of(state));
    let (id_size, id_line) = Type::Id.metrics();
    let line = f64::from(size * id_line / id_size);
    let id = active.then(|| {
        let id = one_line(
            format!("{STABLE_ID_FIELD_PREFIX}{}", passage.id),
            Type::Id,
            size / id_size,
            state.palette.muted,
        );
        // As wide as the column, so its right edge meets the label's, the speaker control's
        // padding hanging past both into the gutter.
        if wide { id.end() } else { id }.width(if wide { grid.margin } else { grid.measure }.px())
    });
    let reach = if wide && active { ID_GAP + line } else { 0. };
    hang_below(
        flex_col((content, id))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(ID_GAP.px()),
        reach,
    )
}

/// A stacked reply's route, or the destination control while it is written, across `width`.
pub(super) fn routes(
    state: &Writer,
    passage: &Passage,
    active: bool,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    // Options, not one Either, so the route keeps its own baselines in the row.
    flex_row((
        route(state, passage, active, width),
        choice(state, passage, active, width),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .width(width.px())
}

/// A resting reply's route; the reply being written shows its [`choice`] instead.
pub(super) fn route(
    state: &Writer,
    passage: &Passage,
    active: bool,
    width: f64,
) -> Option<impl WidgetView<Writer> + use<>> {
    if active {
        return None;
    }
    let PassageKind::Choice {
        destination: Some(destination),
    } = &passage.kind
    else {
        return None;
    };
    if destination == "END" {
        return None;
    }
    Some(route_to(state, destination, width))
}

/// The destination control of the reply being written.
pub(super) fn choice(
    state: &Writer,
    passage: &Passage,
    active: bool,
    width: f64,
) -> Option<impl WidgetView<Writer> + use<>> {
    (active && matches!(passage.kind, PassageKind::Choice { .. }))
        .then(|| super::choice::destination(state, passage, width))
}

/// A destination as a button in the manuscript's destination type: pressing it shows or hides
/// the destination beat's opening beneath the row.
pub(in crate::view::reading) fn route_to(
    state: &Writer,
    destination: &str,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let target = destination.to_owned();
    let size = Sheet::Destination.size(Page::of(state));
    // The chevron is drawn for Control text; it scales to the destination's size.
    let glyph = size / Type::Control.metrics().0;
    sized_box(styled(
        ControlStyle::Action,
        button(
            route_toggle(
                (glyph, state.ui_scale()),
                state.destination_expanded(destination),
                state.palette.teal_text,
                size,
                // Sized with the manuscript it belongs to, not with the interface chrome.
                Sheet::Destination
                    .label(Page::of(state), display_name(destination))
                    .prop(LineBreaking::WordWrap)
                    // Fill the content box so the text starts at the column's left edge.
                    .width(
                        (width - 2. * control_inset(state.ui_scale()) - disclosure_width(glyph))
                            .max(24.)
                            .px(),
                    )
                    .color(state.palette.teal_text),
            ),
            move |state: &mut Writer| state.toggle_destination_preview(target.clone()),
        )
        .expanded(state.destination_expanded(destination))
        .accessible_label(route_name(&display_name(destination)))
        .disabled(state.phase() != Phase::Writing || state.work.busy() || state.navigation.busy())
        .background_color(state.palette.surface),
    ))
    .width(width.px())
}
