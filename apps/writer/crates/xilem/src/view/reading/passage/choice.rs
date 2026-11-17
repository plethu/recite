//! The passage being written offers its choice on the label itself: a line's speaker in the
//! margin, a reply's destination in its column. Pressing the label hangs the choice's slip from
//! it, and the label reports itself expanded while the slip is open. Both keep the resting
//! labels' faces and places, so choosing a passage moves nothing.
use super::*;
use crate::app::keyboard::PickerAnchor;
use crate::presentation::{Type, label_inset, material};
use recite_writer_app::Opener;
use recite_writer_model::RouteOwner;
use xilem::masonry::layout::Dim;
use xilem::masonry::properties::Dimensions;
use xilem::view::MainAxisAlignment;

/// What the margin shows for a line with no speaker while it is written.
const NO_SPEAKER: &str = "SPEAKER";
/// What a reply's destination column shows while it is written and leads nowhere.
const NO_DESTINATION: &str = "No destination";

fn closed(state: &Writer) -> bool {
    state.phase() != Phase::Writing
        || state.session.read_only()
        || state.navigation.busy()
        || state.work.busy()
}

/// The margin column of the line being written: its speaker label is the control that opens
/// the speaker choice. Where the margin is a column, the control is [`label_inset`] wider than
/// it, and the row's gutter that much narrower.
pub(super) fn speaker(
    state: &Writer,
    grid: &Grid,
    passage: &Passage,
) -> impl WidgetView<Writer> + use<> {
    let speaker = match &passage.kind {
        PassageKind::Dialogue { speaker } => speaker.clone(),
        PassageKind::Choice { .. } => None,
    };
    let wide = grid.margin > 0.;
    let inset = label_inset();
    let label = Sheet::Label.label(
        Page::of(state),
        speaker
            .as_deref()
            .map_or_else(|| NO_SPEAKER.to_owned(), str::to_uppercase),
    );
    let name = speaker.as_deref().map_or_else(
        || "Choose a speaker…".to_owned(),
        |speaker| format!("Change speaker, {speaker}…"),
    );
    let label = label
        .color(if speaker.is_some() {
            state.palette.accent
        } else {
            state.palette.muted
        })
        .prop(LineBreaking::Clip);
    // The label centred in the least target, and across the column with its words set against
    // the gutter like a resting label's. The row aligns the control by the label's baseline.
    let face = flex_col((label,))
        .main_axis_alignment(MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::End)
        .dims(
            Dimensions::AUTO
                .with_width(if wide {
                    Dim::from((grid.margin - 2. * inset).px())
                } else {
                    Dim::FitContent
                })
                .with_height(Page::of(state).target().px()),
        );
    let id = passage.id.clone();
    let opener = Opener::Speaker(id.clone());
    flex_row((
        FlexSpacer::Flex(if wide { 1. } else { 0. }),
        // The control's padding hangs past the column, so its words keep the label's place.
        material::focus_ring(
            Some(state.palette.accent),
            styled(
                ControlStyle::Label,
                button(face, move |state: &mut Writer| {
                    state.choose_speaker(id.clone())
                })
                .expanded(state.choosing(&opener))
                .accessible_label(name)
                .disabled(closed(state))
                .prop(PickerAnchor(true)),
            ),
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::FirstBaseline)
    .gap(0.px())
    // The control's padding hangs into the gutter, so its words keep the label's place.
    .width(
        if wide {
            grid.margin + inset
        } else {
            grid.measure
        }
        .px(),
    )
}

/// The destination of the reply being written, in its route's face: the control that opens the
/// destination choice. A reply that ends the conversation or leads nowhere says so.
pub(super) fn destination(
    state: &Writer,
    passage: &Passage,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let destination = match &passage.kind {
        PassageKind::Choice { destination } => destination.clone(),
        PassageKind::Dialogue { .. } => None,
    };
    let (text, name, color) = match destination.as_deref() {
        Some("END") => (
            "End conversation".to_owned(),
            "Change destination, End conversation…".to_owned(),
            state.palette.teal_text,
        ),
        Some(target) => (
            display_name(target),
            format!("Change destination, {}…", display_name(target)),
            state.palette.teal_text,
        ),
        None => (
            NO_DESTINATION.to_owned(),
            "Choose a destination…".to_owned(),
            state.palette.muted,
        ),
    };
    let size = Sheet::Destination.size(Page::of(state));
    // The arrow is drawn for Control text; it scales to the destination's size.
    let glyph = size / Type::Control.metrics().0;
    let owner = RouteOwner::Reply(passage.id.clone());
    let opener = Opener::Destination(owner.clone());
    sized_box(styled(
        ControlStyle::Action,
        button(
            route_toggle(
                (glyph, state.ui_scale()),
                false,
                color,
                size,
                Sheet::Destination
                    .label(Page::of(state), text)
                    .prop(LineBreaking::WordWrap)
                    .width(
                        (width - 2. * control_inset(state.ui_scale()) - disclosure_width(glyph))
                            .max(24.)
                            .px(),
                    )
                    .color(color),
            ),
            move |state: &mut Writer| state.choose_destination(owner.clone()),
        )
        .expanded(state.choosing(&opener))
        .accessible_label(name)
        .disabled(closed(state))
        .background_color(state.palette.surface)
        .prop(PickerAnchor(true)),
    ))
    .width(width.px())
}
