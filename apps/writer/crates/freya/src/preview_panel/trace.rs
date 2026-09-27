//! Human-readable runtime evidence, drawn from the run that delivered the text.
use crate::{
    design::tokens as t,
    messages::{MsgId, text},
};
use freya::prelude::*;
use recite_runtime::{
    ConditionValue,
    localisation::{LocaleLookupOutcome, PluralResolutionOutcome},
    preview::{
        PreviewConditionArgument, PreviewConditionQuery, PreviewConditionResult, PreviewEvent,
        PreviewTrace,
    },
};

pub(super) fn render(trace: &PreviewTrace) -> Element {
    let mut body = rect().width(Size::fill()).spacing(t::SPACE_MD);
    for lookup in trace.localized_lookups() {
        body = body.child(
            label()
                .text(lookup.source_text.clone())
                .font_size(t::prose_size()),
        );
        for attempt in &lookup.attempts {
            body = body.child(row(
                &attempt.locale,
                &attempt.context,
                if attempt.outcome == LocaleLookupOutcome::Matched {
                    MsgId::WriterLookupMatched
                } else {
                    MsgId::WriterLookupMissing
                },
            ));
        }
        if lookup.matched_locale.is_none() {
            body = body.child(label().text(text(MsgId::WriterSourceOnly)));
        }
    }
    for (_, lookup) in trace.plural_lines() {
        body = body.child(
            label()
                .text(format!("{} · {}", lookup.plural_source_text, lookup.count))
                .font_size(t::prose_size()),
        );
        for attempt in &lookup.attempts {
            body = body.child(row(
                &attempt.locale,
                &attempt.context,
                if attempt.outcome == PluralResolutionOutcome::Matched {
                    MsgId::WriterLookupMatched
                } else {
                    MsgId::WriterLookupMissing
                },
            ));
        }
        let selected = lookup
            .matched_locale
            .as_deref()
            .map_or_else(|| text(MsgId::WriterSourceOnly), str::to_owned);
        body = body.child(label().text(format!(
            "{selected} · {} {}",
            text(MsgId::WriterPluralForm),
            lookup.selected_arm + 1
        )));
    }
    for event in trace.events() {
        let caption = match event {
            PreviewEvent::ConditionRequested(request) => format!(
                "{} · {}",
                text(MsgId::WriterConditionInput),
                query(request.query())
            ),
            PreviewEvent::ConditionResult { request, result } => {
                let result = match result {
                    PreviewConditionResult::Value(ConditionValue::Bool(value)) => text(if *value {
                        MsgId::WriterTrue
                    } else {
                        MsgId::WriterFalse
                    }),
                    PreviewConditionResult::Value(ConditionValue::EnumVariant(value)) => {
                        value.clone()
                    }
                    PreviewConditionResult::Failed { reason } => reason.clone(),
                    _ => continue,
                };
                format!("{} → {result}", query(request.query()))
            }
            PreviewEvent::EffectRequested(effect) => {
                format!("{} · {}", text(MsgId::WriterTrialEffect), effect.function)
            }
            PreviewEvent::DeferredEffectScheduled(effect) => {
                format!("{} · {}", text(MsgId::WriterTrialDeferred), effect.function)
            }
            PreviewEvent::EffectAcknowledged { effect_id, ack } => format!(
                "{} · {}",
                effect_id.as_str(),
                match ack {
                    recite_runtime::EffectAck::Completed => text(MsgId::WriterAcknowledge),
                    recite_runtime::EffectAck::Failed { reason } => reason.clone(),
                }
            ),
            PreviewEvent::ChoiceSelected { choice_id, .. } => format!(
                "{} · {}",
                text(MsgId::WriterTrialChoice),
                choice_id.as_str()
            ),
            PreviewEvent::End { .. } => text(MsgId::WriterTrialEnded),
            PreviewEvent::Error(error) => error.to_string(),
            _ => continue,
        };
        body = body.child(label().text(caption));
    }
    body.into_element()
}
fn row(locale: &str, context: &str, outcome: MsgId) -> Element {
    let variant = context.split_once('&').map_or_else(
        || text(MsgId::WriterDefaultWording),
        |(_, variant)| variant.into(),
    );
    label()
        .text(format!("{locale} · {variant} · {}", text(outcome)))
        .font_size(t::small())
        .into_element()
}
pub(super) fn query(query: &PreviewConditionQuery) -> String {
    let values = query
        .arguments()
        .iter()
        .map(|value| match value {
            PreviewConditionArgument::Identifier(value)
            | PreviewConditionArgument::String(value) => value.clone(),
            PreviewConditionArgument::Integer(value) => value.to_string(),
            PreviewConditionArgument::Float(value) => value.to_string(),
            PreviewConditionArgument::Boolean(value) => text(if *value {
                MsgId::WriterTrue
            } else {
                MsgId::WriterFalse
            }),
            _ => String::new(),
        })
        .collect::<Vec<_>>();
    format!("{}({})", query.function(), values.join(", "))
}

pub(super) fn effect(effect: &recite_runtime::DialogueEffectRequest) -> String {
    let values = effect
        .args
        .iter()
        .map(|value| match value {
            recite_runtime::DialogueEffectArgument::Identifier(v)
            | recite_runtime::DialogueEffectArgument::String(v) => v.clone(),
            recite_runtime::DialogueEffectArgument::Integer(v) => v.to_string(),
            recite_runtime::DialogueEffectArgument::Float(v) => v.to_string(),
            recite_runtime::DialogueEffectArgument::Boolean(v) => text(if *v {
                MsgId::WriterTrue
            } else {
                MsgId::WriterFalse
            }),
        })
        .collect::<Vec<_>>();
    format!("{}({})", effect.function, values.join(", "))
}
