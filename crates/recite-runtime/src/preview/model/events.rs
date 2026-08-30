use recite_core::{BlockId, ChoiceId, CompiledAssetId, EffectId, LocaleId};

use crate::{DialogueChoice, DialogueEffectRequest, DialogueEvent, DialogueLine, EffectAck};

use super::api::{
    ConditionAnswer, PreviewConditionRequest, PreviewConditionResult, PreviewPromptIdentity,
};
use super::errors::PreviewError;

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewPrompt {
    pub(crate) identity: PreviewPromptIdentity,
    pub(crate) line: Option<DialogueLine>,
    pub(crate) choices: Vec<DialogueChoice>,
}

impl PreviewPrompt {
    #[must_use]
    pub fn identity(&self) -> &PreviewPromptIdentity {
        &self.identity
    }

    #[must_use]
    pub fn line(&self) -> Option<&DialogueLine> {
        self.line.as_ref()
    }

    #[must_use]
    pub fn choices(&self) -> &[DialogueChoice] {
        &self.choices
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum PreviewEvent {
    ConditionRequested(PreviewConditionRequest),
    ConditionResult {
        request: PreviewConditionRequest,
        result: PreviewConditionResult,
    },
    Line(DialogueLine),
    Prompt(PreviewPrompt),
    ChoiceSelected {
        prompt: PreviewPromptIdentity,
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
    Error(PreviewError),
}

impl PreviewEvent {
    pub(crate) fn from_dialogue_event(event: DialogueEvent, block: BlockId) -> Self {
        match event {
            DialogueEvent::Line(line) => Self::Line(line),
            DialogueEvent::Prompt { line, choices } => {
                let identity = PreviewPromptIdentity {
                    block,
                    line: line.as_ref().map(|line| line.id.clone()),
                    choices: choices.iter().map(|choice| choice.id.clone()).collect(),
                };
                Self::Prompt(PreviewPrompt {
                    identity,
                    line,
                    choices,
                })
            }
            DialogueEvent::Effect(effect) => Self::EffectRequested(effect),
            DialogueEvent::End { deferred_effects } => Self::End { deferred_effects },
        }
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewTrace {
    locale: Option<LocaleId>,
    variant: Option<String>,
    events: Vec<PreviewEvent>,
}

impl PreviewTrace {
    pub(crate) fn new(locale: Option<LocaleId>, variant: Option<String>) -> Self {
        Self {
            locale,
            variant,
            events: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, event: PreviewEvent) {
        self.events.push(event);
    }

    #[must_use]
    pub fn locale(&self) -> Option<&LocaleId> {
        self.locale.as_ref()
    }

    #[must_use]
    pub fn variant(&self) -> Option<&str> {
        self.variant.as_deref()
    }

    #[must_use]
    pub fn events(&self) -> &[PreviewEvent] {
        &self.events
    }
}

/// User-facing transcript projection. Condition control traffic and runtime
/// errors remain in [`PreviewTrace`] rather than being duplicated here.
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
            PreviewEvent::ConditionRequested(_) | PreviewEvent::ConditionResult { .. } => return,
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
