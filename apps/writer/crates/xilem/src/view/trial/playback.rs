use super::*;
use recite_writer_model::{ConditionExpectedType, ConditionValue, EffectAck};
use recite_writer_session::FileError;
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
            text_button(scale, choice.text.clone(), move |state: &mut Writer| {
                let result = state.session.advance_preview(Some(index));
                report(state, result);
            })
            .disabled(disabled)
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
    flex_col((
        (!page.ended && !page.text.is_empty()).then(|| {
            flex_row((
                sized_box(manuscript_text(state.reading_size(), page.text.clone()))
                    .width(state.script_width().px()),
            ))
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
            .gap(8.px())
        }),
        flex_col(effects).gap(8.px()),
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
            .gap(8.px())
        }),
        page.waiting_effect.as_ref().map(|_| {
            flex_row((
                text_button(scale, "Acknowledge effect", |state: &mut Writer| {
                    let result = state.session.acknowledge_preview(EffectAck::Completed);
                    report(state, result);
                })
                .disabled(disabled),
                text_button(scale, "Report effect failure", |state: &mut Writer| {
                    let result = state.session.acknowledge_preview(EffectAck::Failed {
                        reason: "Simulated preview failure".into(),
                    });
                    report(state, result);
                })
                .disabled(disabled),
            ))
            .gap(8.px())
        }),
        flex_col(choices)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px()),
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
            .gap(8.px())
        }),
        can_continue.then(|| {
            text_button(scale, "Continue preview", |state: &mut Writer| {
                let result = state.session.advance_preview(None);
                report(state, result);
            })
            .disabled(disabled)
        }),
        page.ended.then(|| label(scale, "Conversation ended")),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
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
                        report(state, result);
                    },
                )
                .disabled(disabled)
            }))
            .gap(8.px()),
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
                    report(state, result);
                })
                .disabled(disabled || state.trial.answer.trim().is_empty()),
            ))
            .gap(8.px()),
        ),
    }
}

fn report(state: &mut Writer, result: Result<(), FileError>) {
    state.trial.choice_page = 0;
    state.trial.effect_page = 0;
    state.trial.status = match result {
        Ok(()) => String::new(),
        Err(error) => error.to_string(),
    };
}
