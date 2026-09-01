use recite_core::{BlockId, ChoiceId, CompiledAssetId, EffectId, LocaleId};

use crate::{DialogueEffectRequest, DialogueLine, EffectAck};

use super::{ConditionAnswer, PreviewConditionResult, PreviewEvent, PreviewPrompt};

/// User-facing transcript projection. Condition control traffic, tentative
/// choice acceptance, and runtime errors remain in [`crate::PreviewTrace`]
/// rather than being duplicated here.
#[non_exhaustive]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PreviewTranscript {
    events: Vec<PreviewTranscriptEvent>,
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum PreviewTranscriptEvent {
    Line(DialogueLine),
    Prompt(PreviewPrompt),
    ChoiceSelected {
        choice_id: ChoiceId,
    },
    EffectRequested(DialogueEffectRequest),
    DeferredEffectScheduled(DialogueEffectRequest),
    EffectAcknowledged {
        effect_id: EffectId,
        ack: EffectAck,
    },
    End {
        deferred_effects: Vec<DialogueEffectRequest>,
    },
    Restarted {
        block: Option<BlockId>,
        locale: Option<LocaleId>,
    },
    Restored,
    RestartRequired {
        active_asset: CompiledAssetId,
        replacement_asset: CompiledAssetId,
    },
}

impl PreviewTranscript {
    pub(crate) fn push(&mut self, event: &PreviewEvent) {
        let event = match event {
            PreviewEvent::ConditionRequested(_)
            | PreviewEvent::ConditionResult { .. }
            | PreviewEvent::ChoiceAccepted { .. } => return,
            PreviewEvent::Line(line) => PreviewTranscriptEvent::Line(line.clone()),
            PreviewEvent::Prompt(prompt) => PreviewTranscriptEvent::Prompt(prompt.clone()),
            PreviewEvent::ChoiceSelected { choice_id, .. } => {
                PreviewTranscriptEvent::ChoiceSelected {
                    choice_id: choice_id.clone(),
                }
            }
            PreviewEvent::EffectRequested(effect) => {
                PreviewTranscriptEvent::EffectRequested(effect.clone())
            }
            PreviewEvent::DeferredEffectScheduled(effect) => {
                PreviewTranscriptEvent::DeferredEffectScheduled(effect.clone())
            }
            PreviewEvent::EffectAcknowledged { effect_id, ack } => {
                PreviewTranscriptEvent::EffectAcknowledged {
                    effect_id: effect_id.clone(),
                    ack: ack.clone(),
                }
            }
            PreviewEvent::End { deferred_effects } => PreviewTranscriptEvent::End {
                deferred_effects: deferred_effects.clone(),
            },
            PreviewEvent::Restarted { block, locale } => PreviewTranscriptEvent::Restarted {
                block: block.clone(),
                locale: locale.clone(),
            },
            PreviewEvent::Restored => PreviewTranscriptEvent::Restored,
            PreviewEvent::RestartRequired {
                active_asset,
                replacement_asset,
                ..
            } => PreviewTranscriptEvent::RestartRequired {
                active_asset: active_asset.clone(),
                replacement_asset: replacement_asset.clone(),
            },
            PreviewEvent::Error(_) => return,
        };
        self.events.push(event);
    }

    #[must_use]
    pub fn events(&self) -> &[PreviewTranscriptEvent] {
        &self.events
    }
}

impl From<&ConditionAnswer> for PreviewConditionResult {
    fn from(answer: &ConditionAnswer) -> Self {
        Self::from_answer(answer)
    }
}
