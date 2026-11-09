//! A passage occupies the same manuscript row while its single draft is edited.
use super::*;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::print;
use crate::reading::PassageNumber;
use recite_core::STABLE_ID_FIELD_PREFIX;
use recite_writer_model::{Passage, PassageKind};
use xilem::TextAlign;
use xilem::masonry::kurbo::Axis;
use xilem::masonry::properties::LineBreaking;
use xilem::view::button;
#[cfg(test)]
mod tests;

pub(super) fn row(
    state: &Writer,
    number: PassageNumber,
    passage: &Passage,
    reply: Option<usize>,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let caption_size = 0.6 * state.reading_size();
    let wide = width >= 560. * f64::from(scale);
    let margin = if wide {
        (96. * f64::from(scale)).min(width * 0.15)
    } else {
        0.
    };
    let destination = if reply.is_some() && wide {
        (148. * f64::from(scale)).min(width * 0.25)
    } else {
        0.
    };
    let number_width = if reply.is_some() {
        16. * f64::from(scale)
    } else {
        0.
    };
    let number_gap = if reply.is_some() { 8. } else { 0. };
    let measure =
        (width - margin - destination - if wide { 16. } else { 0. } - number_width - number_gap)
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
    let stacked_reply = !wide
        && matches!(&passage.kind, PassageKind::Choice { destination: Some(target) } if target != "END");
    let content_gap = if !wide && speaker.is_empty() { 0. } else { 8. };
    flex_col((
        flex_row((
            indexed_stack((
                print::caps(scale, speaker.clone())
                    .text_size(caption_size)
                    .letter_spacing(0.066 * state.reading_size())
                    .text_alignment(if wide {
                        TextAlign::End
                    } else {
                        TextAlign::Start
                    })
                    .color(state.palette.accent)
                    .width(if wide { margin } else { width }.px()),
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
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(number_gap.px()),
            (!stacked_reply)
                .then(|| route(state, passage, if wide { destination } else { width }))
                .flatten(),
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
                label(
                    state.ui_scale(),
                    format!(
                        "{} {}",
                        if state.destination_expanded(destination) {
                            "▾"
                        } else {
                            "→"
                        },
                        display_name(destination)
                    ),
                )
                .color(state.palette.teal_text),
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
