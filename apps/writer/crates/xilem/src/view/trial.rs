//! Source-language runtime controls; all traversal stays in the shared model.
use super::*;
use recite_writer_model::{ConditionExpectedType, ConditionValue, EffectAck, PreviewPage};
mod captions;

pub(super) fn panel(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let closing = state.phase != Phase::Writing;
    let busy = state.trial.busy();
    let model = state.project.current().model();
    let page = model.preview_page().filter(|_| state.trial.visible);
    flex_col((
        label("Conversation preview").text_size(22.),
        flex_row((
            text_button("Start / restart preview", Writer::start_trial).disabled(
                closing
                    || busy
                    || state.reading.script().is_err()
                    || state.reading.script().is_ok_and(<[_]>::is_empty),
            ),
            text_button("Cancel preparation", |state: &mut Writer| {
                state.trial.cancel()
            })
            .disabled(closing || !busy),
            text_button("Hide preview", |state: &mut Writer| {
                state.trial.cancel();
                state.trial.visible = false;
            })
            .disabled(closing || !state.trial.visible),
        ))
        .gap(8.px()),
        (state.reading.script().is_err() && !state.trial.visible).then(|| {
            sized_box(prose(
                "Repair this source in your editor before starting a preview.",
            ))
            .width(560.px())
        }),
        state.trial.visible.then(|| {
            flex_col((
                sized_box(prose(state.trial.status.clone())).width(560.px()),
                (page.is_some() && model.preview_stale()).then(|| {
                    sized_box(prose(
                        "This preview uses older inputs. Restart to use your accepted edits.",
                    ))
                    .width(560.px())
                }),
                page.map(|page| playback(state, page, closing || busy)),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(12.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}

fn playback(state: &Writer, page: &PreviewPage, disabled: bool) -> impl WidgetView<Writer> + use<> {
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
            text_button(choice.text.clone(), move |state: &mut Writer| {
                let result = state.project.current_mut().advance_preview(Some(index));
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
        .map(|effect| sized_box(prose(captions::effect(effect))).width(560.px()))
        .collect::<Vec<_>>();
    let can_continue = page.choices.is_empty()
        && !page.ended
        && page.condition.is_none()
        && page.waiting_effect.is_none();
    flex_col((
        (!page.ended && !page.text.is_empty())
            .then(|| sized_box(prose(page.text.clone()).text_size(20.)).width(560.px())),
        page.condition.as_ref().map(|request| {
            flex_col((
                sized_box(prose(captions::condition(request.query()))).width(560.px()),
                condition(state, request.query().expected_type(), disabled),
            ))
            .gap(8.px())
        }),
        flex_col(effects).gap(8.px()),
        (effect_pages > 1).then(|| {
            flex_row((
                text_button("Previous effect requests", |state: &mut Writer| {
                    state.trial.effect_page = state.trial.effect_page.saturating_sub(1)
                })
                .disabled(disabled || effect_page == 0),
                label(format!("{} of {effect_pages}", effect_page + 1)),
                text_button("Next effect requests", |state: &mut Writer| {
                    state.trial.effect_page += 1
                })
                .disabled(disabled || effect_page + 1 >= effect_pages),
            ))
            .gap(8.px())
        }),
        page.waiting_effect.as_ref().map(|_| {
            flex_row((
                text_button("Acknowledge effect", |state: &mut Writer| {
                    let result = state
                        .project
                        .current_mut()
                        .acknowledge_preview(EffectAck::Completed);
                    report(state, result);
                })
                .disabled(disabled),
                text_button("Report effect failure", |state: &mut Writer| {
                    let result =
                        state
                            .project
                            .current_mut()
                            .acknowledge_preview(EffectAck::Failed {
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
                text_button("Previous preview choices", |state: &mut Writer| {
                    state.trial.choice_page = state.trial.choice_page.saturating_sub(1)
                })
                .disabled(disabled || choice_page == 0),
                label(format!("{} of {choice_pages}", choice_page + 1)),
                text_button("Next preview choices", |state: &mut Writer| {
                    state.trial.choice_page += 1
                })
                .disabled(disabled || choice_page + 1 >= choice_pages),
            ))
            .gap(8.px())
        }),
        can_continue.then(|| {
            text_button("Continue preview", |state: &mut Writer| {
                let result = state.project.current_mut().advance_preview(None);
                report(state, result);
            })
            .disabled(disabled)
        }),
        page.ended.then(|| label("Conversation ended")),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}

fn condition(
    state: &Writer,
    expected: ConditionExpectedType,
    disabled: bool,
) -> impl WidgetView<Writer> + use<> {
    match expected {
        ConditionExpectedType::Bool => Either::A(
            flex_row([true, false].map(|value| {
                text_button(
                    if value { "Answer true" } else { "Answer false" },
                    move |state: &mut Writer| {
                        let result = state
                            .project
                            .current_mut()
                            .answer_preview(ConditionValue::Bool(value));
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
                    .prop(xilem::masonry::properties::TextAreaLabel::new(
                        "Condition enum value",
                    )),
                )
                .width(240.px()),
                text_button("Answer condition", |state: &mut Writer| {
                    let value = ConditionValue::EnumVariant(state.trial.answer.trim().to_owned());
                    let result = state.project.current_mut().answer_preview(value);
                    report(state, result);
                })
                .disabled(disabled || state.trial.answer.trim().is_empty()),
            ))
            .gap(8.px()),
        ),
    }
}

fn report(state: &mut Writer, result: Result<(), recite_writer_model::WorkbenchError>) {
    state.trial.choice_page = 0;
    state.trial.effect_page = 0;
    state.trial.status = match result {
        Ok(()) => "Preview uses the accepted source snapshot.".into(),
        Err(error) => error.to_string(),
    };
}
