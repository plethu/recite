use recite_core::{BlockId, ChoiceId, CompiledAssetId, EffectId, LocaleId};

use crate::{DialogueChoice, DialogueEffectRequest, DialogueEvent, DialogueLine, EffectAck};
use crate::{LocalizedLookupTrace, PluralLineTrace};
use std::collections::BTreeMap;

use super::api::{PreviewConditionRequest, PreviewConditionResult, PreviewPromptIdentity};
use super::errors::PreviewError;

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewPrompt {
    pub(crate) identity: PreviewPromptIdentity,
    pub(crate) line: Option<DialogueLine>,
    pub(crate) choices: Vec<DialogueChoice>,
}

impl PreviewPrompt {
    pub(crate) fn from_parts(
        identity: PreviewPromptIdentity,
        line: Option<DialogueLine>,
        choices: Vec<DialogueChoice>,
    ) -> Self {
        Self {
            identity,
            line,
            choices,
        }
    }

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
    plural_lines: BTreeMap<String, PluralLineTrace>,
    localized_lookups: BTreeMap<String, LocalizedLookupTrace>,
}

impl PreviewTrace {
    pub(crate) fn new(locale: Option<LocaleId>, variant: Option<String>) -> Self {
        Self {
            locale,
            variant,
            events: Vec::new(),
            plural_lines: BTreeMap::new(),
            localized_lookups: BTreeMap::new(),
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

    #[must_use]
    pub fn plural_line(&self, id: &str) -> Option<&PluralLineTrace> {
        self.plural_lines.get(id)
    }

    pub fn localized_lookups(&self) -> impl Iterator<Item = &LocalizedLookupTrace> {
        self.localized_lookups.values()
    }

    pub(crate) fn merge_runtime_trace(&mut self, trace: &crate::DialogueTrace) {
        self.plural_lines.extend(trace.plural_lines());
        self.localized_lookups.extend(
            trace
                .localized_lookups()
                .into_iter()
                .map(|entry| (format!("{:?}:{}", entry.domain, entry.id), entry)),
        );
    }
}
