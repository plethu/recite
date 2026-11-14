//! A passage occupies the same manuscript row while its single draft is edited.
use super::*;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::{disclosure_width, print, route_toggle};
use crate::reading::PassageNumber;
use recite_core::STABLE_ID_FIELD_PREFIX;
use recite_writer_model::{Passage, PassageKind};
use xilem::TextAlign;
use xilem::masonry::kurbo::Axis;
use xilem::masonry::properties::LineBreaking;
use xilem::view::button;
#[cfg(test)]
mod tests;

/// The destination column at the end of a reply's measure.
const DESTINATION: f64 = 168.;
/// Below this row width, destinations wrap under their reply instead.
const DESTINATION_MEASURE: f64 = 420.;
/// Destination text relative to the reading size: 14px beside 20px prose.
const DESTINATION_TEXT: f32 = 0.7;

/// The speaker margin left of the text column; narrow rows set speakers above instead.
fn speaker_margin(state: &Writer, width: f64) -> f64 {
    let scale = f64::from(state.ui_scale());
    if width >= 560. * scale {
        (96. * scale).min(width * 0.15)
    } else {
        0.
    }
}

/// Where dialogue text starts in a row of `width`, for actions that line up with it.
pub(in crate::view) fn text_column(state: &Writer, width: f64) -> f64 {
    let margin = speaker_margin(state, width);
    if margin > 0. { margin + 8. } else { 0. }
}

fn reply_count(replies: usize) -> String {
    if replies == 1 {
        "1 reply".to_owned()
    } else {
        format!("{replies} replies")
    }
}

pub(super) fn row(
    state: &Writer,
    number: PassageNumber,
    passage: &Passage,
    reply: Option<usize>,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let caption_size = 0.6 * state.reading_size();
    let margin = speaker_margin(state, width);
    let wide = margin > 0.;
    // Destinations share one left-aligned column at the end of the measure, until the measure is
    // too narrow to keep both readable.
    let column = reply.is_some() && width >= DESTINATION_MEASURE;
    let destination = if column { DESTINATION } else { 0. };
    let number_width = if reply.is_some() {
        16. * f64::from(scale)
    } else {
        0.
    };
    let number_gap = if reply.is_some() { 8. } else { 0. };
    let measure = (width
        - margin
        - if wide { 8. } else { 0. }
        - number_width
        - number_gap
        - if column { destination + 8. } else { 0. })
    .max(120.);
    let speaker = match &passage.kind {
        PassageKind::Dialogue { speaker } => speaker.as_deref().unwrap_or("").to_uppercase(),
        PassageKind::Choice { .. } => {
            if reply == Some(1) {
                "REPLIES".into()
            } else {
                String::new()
            }
        }
    };
    let active = matches!(state.session.model().view(), View::Passage(id) if id == &passage.id);
    let stacked_reply = !column
        && matches!(&passage.kind, PassageKind::Choice { destination: Some(target) } if target != "END");
    let content_gap = if !wide && speaker.is_empty() { 0. } else { 8. };
    let alignment = if wide {
        TextAlign::End
    } else {
        TextAlign::Start
    };
    flex_col((
        flex_row((
            indexed_stack((
                flex_col((
                    print::caps(scale, speaker.clone())
                        .text_size(caption_size)
                        .letter_spacing(0.066 * state.reading_size())
                        .text_alignment(alignment)
                        .color(state.palette.accent)
                        .width(if wide { margin } else { width }.px()),
                    (reply == Some(1)).then(|| {
                        // 12px beside 20px prose, scaled with the manuscript like its caption.
                        label(1., reply_count(state.reading.replies))
                            .text_size(caption_size)
                            .text_alignment(alignment)
                            .color(state.palette.muted)
                            .width(if wide { margin } else { width }.px())
                    }),
                ))
                .gap(2.px()),
                flex_col(()).dims((0.px(), 0.px())),
            ))
            .active(usize::from(!wide && speaker.is_empty())),
            flex_row((
                reply.map(|number| {
                    print::meta(scale, number.to_string())
                        .text_size(caption_size)
                        .width(number_width.px())
                        .color(state.palette.muted)
                }),
                super::super::manuscript::passage_editor(state, passage, number, measure),
                column
                    .then(|| route(state, passage, destination))
                    .flatten()
                    .map(|route| sized_box(route).padding(Padding::left(8.px()))),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(number_gap.px()),
        ))
        .direction(if wide {
            Axis::Horizontal
        } else {
            Axis::Vertical
        })
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(content_gap.px()),
        metadata(
            state,
            passage,
            active,
            if stacked_reply {
                CaptionPlacement::BesideDestination { width }
            } else {
                CaptionPlacement::Under { margin }
            },
        ),
        active
            .then(|| super::refusal::panel(state, measure))
            .flatten()
            .map(|notice| {
                sized_box(notice).padding(Padding::left(
                    (margin + if wide { 8. } else { 0. } + number_width + number_gap).px(),
                ))
            }),
        match &passage.kind {
            PassageKind::Choice {
                destination: Some(target),
            } => super::super::reference::destination(state, target, width),
            _ => None,
        },
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(4.px())
}

/// Source labels either reserve their own line or share a stacked destination's target height.
enum CaptionPlacement {
    Under { margin: f64 },
    BesideDestination { width: f64 },
}

fn metadata(
    state: &Writer,
    passage: &Passage,
    active: bool,
    placement: CaptionPlacement,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let caption_size = 0.6 * state.reading_size();
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
                    print::meta(scale, metadata)
                        .text_size(caption_size)
                        .prop(LineBreaking::Clip)
                        .color(state.palette.muted)
                        .width(caption_width.px()),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(8.px())
                .width(width.px()),
            )
        }
        CaptionPlacement::Under { margin } => Either::B(
            sized_box(
                flex_row((
                    flex_col(()).dims((margin.px(), 0.px())),
                    print::meta(scale, metadata)
                        .text_size(caption_size)
                        .color(state.palette.muted),
                ))
                .gap(8.px()),
            )
            .height((0.9 * f64::from(state.reading_size())).px()),
        ),
    }
}

fn route(state: &Writer, passage: &Passage, width: f64) -> Option<impl WidgetView<Writer> + use<>> {
    let PassageKind::Choice {
        destination: Some(destination),
    } = &passage.kind
    else {
        return None;
    };
    if destination == "END" {
        return None;
    }
    let target = destination.clone();
    Some(
        sized_box(styled(
            ControlStyle::Action,
            button(
                route_toggle(
                    DESTINATION_TEXT * state.reading_size() / 14.,
                    state.destination_expanded(destination),
                    state.palette.teal_text,
                    DESTINATION_TEXT * state.reading_size(),
                    label(1., display_name(destination))
                        // Sized with the manuscript it belongs to, not with the interface chrome.
                        .text_size(DESTINATION_TEXT * state.reading_size())
                        .prop(LineBreaking::WordWrap)
                        // Fill the content box so the text starts at the column's left edge.
                        .width(
                            (width
                                - 2. * (12. * f64::from(state.ui_scale()) + 1.)
                                - disclosure_width(DESTINATION_TEXT * state.reading_size() / 14.))
                            .max(24.)
                            .px(),
                        )
                        .color(state.palette.teal_text),
                ),
                move |state: &mut Writer| state.toggle_destination_preview(target.clone()),
            )
            .disabled(
                state.phase() != Phase::Writing || state.work.busy() || state.navigation.busy(),
            )
            .background_color(state.palette.surface),
        ))
        .width(width.px()),
    )
}
