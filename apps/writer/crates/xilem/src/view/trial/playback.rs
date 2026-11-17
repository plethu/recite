use super::*;
use crate::WriterLayout;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::row::named_row;
use crate::presentation::sheet_text::sheet_text;
use crate::presentation::{Page, Sheet, print, revealed, space};
use recite_writer_model::{ConditionExpectedType, ConditionValue, EffectAck};
use xilem::masonry::properties;

pub(super) fn playback(
    state: &Writer,
    page: &PreviewPage,
    disabled: bool,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    const PAGE: usize = 16;
    let choice_pages = page.choices.len().div_ceil(PAGE).max(1);
    let choice_page = state.trial.choice_page.min(choice_pages - 1);
    let effect_pages = page.effects.len().div_ceil(PAGE).max(1);
    let effect_page = state.trial.effect_page.min(effect_pages - 1);
    let choices = page
        .choices
        .iter()
        .enumerate()
        .skip(choice_page * PAGE)
        .take(PAGE)
        .map(|(index, choice)| {
            // A numbered reply in the manuscript's type: the number is its digit key.
            styled(
                ControlStyle::Action,
                named_row(
                    choice.text.clone(),
                    flex_row((
                        Sheet::Number
                            .label(Page::of(state), format!("{}", index + 1))
                            .color(state.palette.muted),
                        sheet_text(
                            choice.text.clone(),
                            Sheet::Reply.size(Page::of(state)),
                            state.palette.ink,
                        )
                        .width(state.script_width().px()),
                    ))
                    .cross_axis_alignment(CrossAxisAlignment::FirstBaseline)
                    .gap(space::TEXT.px()),
                    move |state: &mut Writer| state.choose_preview(index),
                )
                .disabled(disabled),
            )
        })
        .collect::<Vec<_>>();
    let effects = page
        .effects
        .iter()
        .skip(effect_page * PAGE)
        .take(PAGE)
        .map(|effect| paragraph(scale, captions::effect(effect), state.script_width()))
        .collect::<Vec<_>>();
    let can_continue = page.choices.is_empty()
        && !page.ended
        && page.condition.is_none()
        && page.waiting_effect.is_none();
    let shown = state.session.model().preview_transcript().len();
    flex_col((
        transcript(state),
        // The current page in full ink, brought into view as each turn joins the transcript.
        (!page.ended && !page.text.is_empty()).then(|| {
            revealed(
                shown,
                flex_row((sized_box(manuscript_text(
                    Sheet::Dialogue.size(Page::of(state)),
                    page.text.clone(),
                ))
                .width(state.script_width().px()),)),
            )
        }),
        page.condition.as_ref().map(|request| {
            flex_col((
                paragraph(
                    scale,
                    captions::condition(request.query()),
                    state.script_width(),
                ),
                condition(state, request.query().expected_type(), disabled),
            ))
            .gap(space::SIBLINGS.px())
        }),
        flex_col(effects).gap(space::SIBLINGS.px()),
        (effect_pages > 1).then(|| {
            flex_row((
                text_button(scale, "Previous effect requests", |state: &mut Writer| {
                    state.trial.effect_page = state.trial.effect_page.saturating_sub(1)
                })
                .disabled(disabled || effect_page == 0),
                label(scale, format!("{} of {effect_pages}", effect_page + 1)),
                text_button(scale, "Next effect requests", |state: &mut Writer| {
                    state.trial.effect_page += 1
                })
                .disabled(disabled || effect_page + 1 >= effect_pages),
            ))
            .gap(space::SIBLINGS.px())
        }),
        page.waiting_effect.as_ref().map(|_| {
            flex_row((
                text_button(scale, "Acknowledge effect", |state: &mut Writer| {
                    let result = state.session.acknowledge_preview(EffectAck::Completed);
                    state.report_trial_step(result);
                })
                .disabled(disabled),
                text_button(scale, "Report effect failure", |state: &mut Writer| {
                    let result = state.session.acknowledge_preview(EffectAck::Failed {
                        reason: "Simulated preview failure".into(),
                    });
                    state.report_trial_step(result);
                })
                .disabled(disabled),
            ))
            .gap(space::SIBLINGS.px())
        }),
        flex_col(choices)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(0.px()),
        (choice_pages > 1).then(|| {
            flex_row((
                text_button(scale, "Previous preview choices", |state: &mut Writer| {
                    state.trial.choice_page = state.trial.choice_page.saturating_sub(1)
                })
                .disabled(disabled || choice_page == 0),
                label(scale, format!("{} of {choice_pages}", choice_page + 1)),
                text_button(scale, "Next preview choices", |state: &mut Writer| {
                    state.trial.choice_page += 1
                })
                .disabled(disabled || choice_page + 1 >= choice_pages),
            ))
            .gap(space::SIBLINGS.px())
        }),
        can_continue.then(|| {
            text_button(scale, "Continue preview", |state: &mut Writer| {
                let result = state.session.advance_preview(None);
                state.report_trial_step(result);
            })
            .disabled(disabled)
        }),
        page.ended.then(|| label(scale, "Conversation ended")),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
}

/// What the preview has already shown, oldest first and muted: each page's lines, and the reply
/// chosen after them as a reply row, with a hairline closing each turn before the next.
fn transcript(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let width = state.script_width();
    let page = Page::of(state);
    let muted = state.palette.muted;
    let turns = state
        .session
        .model()
        .preview_transcript()
        .iter()
        .map(|turn| {
            flex_col((
                (!turn.text.is_empty()).then(|| {
                    sheet_text(turn.text.clone(), Sheet::Dialogue.size(page), muted)
                        .width(width.px())
                }),
                turn.reply.as_ref().map(|reply| {
                    sheet_text(format!("→ {reply}"), Sheet::Reply.size(page), muted)
                        .width(width.px())
                }),
                sized_box(print::hairline(&state.palette)).width(width.px()),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px())
        })
        .collect::<Vec<_>>();
    flex_col(turns)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::TEXT.px())
}

fn condition(
    state: &Writer,
    expected: ConditionExpectedType,
    disabled: bool,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    match expected {
        ConditionExpectedType::Bool => Either::A(
            flex_row([true, false].map(|value| {
                text_button(
                    scale,
                    if value { "Answer true" } else { "Answer false" },
                    move |state: &mut Writer| {
                        let result = state.session.answer_preview(ConditionValue::Bool(value));
                        state.report_trial_step(result);
                    },
                )
                .disabled(disabled)
            }))
            .gap(space::SIBLINGS.px()),
        ),
        ConditionExpectedType::Enum => Either::B(
            flex_row((
                sized_box(
                    text_input(state.trial.answer.clone(), |state: &mut Writer, answer| {
                        state.trial.answer = answer
                    })
                    .disabled(disabled)
                    .prop(properties::TextAreaLabel::new("Condition enum value")),
                )
                .width(240.px()),
                text_button(scale, "Answer condition", |state: &mut Writer| {
                    let value = ConditionValue::EnumVariant(state.trial.answer.trim().to_owned());
                    let result = state.session.answer_preview(value);
                    state.report_trial_step(result);
                })
                .disabled(disabled || state.trial.answer.trim().is_empty()),
            ))
            .gap(space::SIBLINGS.px()),
        ),
    }
}
