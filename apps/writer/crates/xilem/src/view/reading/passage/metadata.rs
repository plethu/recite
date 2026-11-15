//! What sits beside or under a reply's text: its destination button and, while it is edited, its
//! stable id as a source label.
use super::*;

/// Source labels either reserve their own line or share a stacked destination's target height.
pub(super) enum CaptionPlacement {
    Under { margin: f64 },
    BesideDestination { width: f64 },
}

pub(super) fn metadata(
    state: &Writer,
    passage: &Passage,
    active: bool,
    placement: CaptionPlacement,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let caption_size = Sheet::Number.size(state.reading_size());
    let metadata =
        if active && (state.editing_passage(&passage.id) || state.session.model().has_draft()) {
            format!("{STABLE_ID_FIELD_PREFIX}{}", passage.id)
        } else {
            String::new()
        };
    match placement {
        CaptionPlacement::BesideDestination { width } => {
            let caption_width = width * 0.25;
            Either::A(
                flex_row((
                    route(state, passage, (width - caption_width - 8.).max(80.)),
                    print::id(scale, metadata)
                        .text_size(caption_size)
                        .prop(LineBreaking::Clip)
                        .color(state.palette.muted)
                        .width(caption_width.px()),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(space::SIBLINGS.px())
                .width(width.px()),
            )
        }
        CaptionPlacement::Under { margin } => Either::B(
            sized_box(
                flex_row((
                    flex_col(()).dims((margin.px(), 0.px())),
                    print::id(scale, metadata)
                        .text_size(caption_size)
                        .color(state.palette.muted),
                ))
                .gap(space::SIBLINGS.px()),
            )
            // Only clearance below the reply's descenders takes space; the caption itself
            // overhangs into the column gap.
            .padding(Padding::top(CAPTION_CLEARANCE.px()))
            .height(CAPTION_CLEARANCE.px()),
        ),
    }
}

pub(super) fn route(
    state: &Writer,
    passage: &Passage,
    width: f64,
) -> Option<impl WidgetView<Writer> + use<>> {
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

/// A destination as a button in the manuscript's destination type: pressing it shows or hides
/// the destination beat's opening beneath the row.
pub(in crate::view::reading) fn route_to(
    state: &Writer,
    destination: &str,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let target = destination.to_owned();
    let size = Sheet::Destination.size(state.reading_size());
    // The chevron is drawn for Control text; it scales to the destination's size.
    let glyph = size / Type::Control.metrics().0;
    sized_box(styled(
        ControlStyle::Action,
        button(
            route_toggle(
                glyph,
                state.destination_expanded(destination),
                state.palette.teal_text,
                size,
                // Sized with the manuscript it belongs to, not with the interface chrome.
                Sheet::Destination
                    .label(state.reading_size(), display_name(destination))
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
