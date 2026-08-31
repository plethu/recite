use recite_core::{ChoiceId, CompiledDialogue, EffectId};
use recite_runtime::{
    ConditionAnswer, ConditionExpectedType, ConditionValue, DialogueEffectRequest, DialogueLine,
    PreviewConditionRequest, PreviewConditionResult, PreviewPrompt,
};

use crate::error::CliError;
use crate::i18n::MsgId;
use crate::runtime_format::format_effect_arguments;
use crate::tui::{Keymap, PromptMode, TextBuffer, TuiInteractionState};

use super::super::plain_input::condition_query_text;
use super::super::preview::PreviewPlayUi;
use super::TuiPlayUi;
use super::choice::resolve_choice;
use super::state::{
    TuiDeferredEffectRow, TuiDeferredQueueState, TuiPrompt, TuiTranscriptEntry, TuiTranscriptKind,
    finished_interaction, initial_interaction,
};

pub(super) fn condition_prompt(
    expected_type: ConditionExpectedType,
    query: String,
    selected_bool_answer: bool,
    keymap: Keymap,
) -> TuiPrompt {
    match expected_type {
        ConditionExpectedType::Bool => TuiPrompt::Condition {
            query,
            selected: selected_bool_answer,
            interaction: initial_interaction(keymap),
        },
        ConditionExpectedType::Enum => TuiPrompt::EnumCondition {
            query,
            interaction: initial_interaction(keymap),
            input: TextBuffer::default(),
        },
    }
}

impl<B: ratatui::backend::Backend> TuiPlayUi<'_, B> {
    pub(super) fn prepare_condition_prompt(
        &mut self,
        request: &PreviewConditionRequest,
        query: String,
    ) {
        let selected = self.condition_answers.get(&query).copied().unwrap_or(true);
        self.state.prompt = condition_prompt(
            request.query().expected_type(),
            query,
            selected,
            self.settings.keymap,
        );
        self.state.status.clear();
    }
}

impl<B: ratatui::backend::Backend> PreviewPlayUi for TuiPlayUi<'_, B> {
    fn start(&mut self, asset: &CompiledDialogue, block: &str) -> Result<(), CliError> {
        self.state.asset = asset.header.asset_id.as_str().to_owned();
        self.state.block = block.to_owned();
        self.state.status = self.messages.text(MsgId::TuiReady);
        self.render()
    }

    fn line(&mut self, line: &DialogueLine) -> Result<(), CliError> {
        self.state.prompt = TuiPrompt::None;
        self.push(
            TuiTranscriptKind::Line,
            Some(line.id.as_str().to_owned()),
            line.text.clone(),
        )
    }

    fn choice(&mut self, prompt: &PreviewPrompt) -> Result<ChoiceId, CliError> {
        self.prepare_choice_prompt(prompt);
        let selection = self.read_choice_selection()?;
        resolve_choice(&self.messages, selection, prompt)
    }

    fn selected_choice(&mut self, choice_id: &ChoiceId) -> Result<(), CliError> {
        if let TuiPrompt::Choice {
            line: Some(line), ..
        } = &self.state.prompt
        {
            self.state.transcript.push(TuiTranscriptEntry {
                kind: TuiTranscriptKind::Prompt,
                id: Some(line.id.clone()),
                text: line.text.clone(),
            });
        }
        self.push(
            TuiTranscriptKind::Choice,
            Some(choice_id.as_str().to_owned()),
            String::new(),
        )
    }

    fn condition(
        &mut self,
        request: &PreviewConditionRequest,
    ) -> Result<ConditionAnswer, CliError> {
        let query = condition_query_text(request)?;
        self.prepare_condition_prompt(request, query.clone());
        match request.query().expected_type() {
            ConditionExpectedType::Enum => Ok(ConditionAnswer::Value(ConditionValue::EnumVariant(
                self.read_enum_condition_variant()?,
            ))),
            ConditionExpectedType::Bool => {
                let answer = self.read_condition_selection()?;
                self.condition_answers.insert(query, answer);
                Ok(ConditionAnswer::Value(ConditionValue::Bool(answer)))
            }
        }
    }

    fn condition_result(
        &mut self,
        request: &PreviewConditionRequest,
        result: &PreviewConditionResult,
    ) -> Result<(), CliError> {
        let PreviewConditionResult::Value(value) = result else {
            return Ok(());
        };
        let query = condition_query_text(request)?;
        let value = match value {
            ConditionValue::Bool(value) => value.to_string(),
            ConditionValue::EnumVariant(value) => value.clone(),
        };
        self.push(TuiTranscriptKind::Condition, Some(query), value)
    }

    fn effect(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        let args = format_effect_arguments(&effect.args);
        self.state.prompt = TuiPrompt::Effect {
            mode: effect.mode.to_string(),
            id: effect.id.as_str().to_owned(),
            function: effect.function.clone(),
            args: args.clone(),
            interaction: TuiInteractionState::new(PromptMode::Insert),
            input: TextBuffer::default(),
        };
        self.push(
            TuiTranscriptKind::Effect,
            Some(effect.id.as_str().to_owned()),
            self.messages.format(
                MsgId::TuiTranscriptEffectText,
                [
                    ("mode", effect.mode.to_string()),
                    ("function", effect.function.clone()),
                    ("args", args),
                ],
            ),
        )
    }

    fn acknowledge(&mut self, _effect: &DialogueEffectRequest) -> Result<(), CliError> {
        self.state.status.clear();
        self.read_effect_acknowledgement()
    }

    fn acknowledged(&mut self, effect_id: &EffectId) -> Result<(), CliError> {
        self.push(
            TuiTranscriptKind::Ack,
            Some(effect_id.as_str().to_owned()),
            self.messages.text(MsgId::TuiTranscriptCompleted),
        )
    }

    fn deferred_effect_scheduled(
        &mut self,
        effect: &DialogueEffectRequest,
    ) -> Result<(), CliError> {
        self.state.deferred_queue.push(TuiDeferredEffectRow {
            id: effect.id.as_str().to_owned(),
            function: effect.function.clone(),
            args: format_effect_arguments(&effect.args),
        });
        self.state.deferred_queue_state = Some(TuiDeferredQueueState::Scheduled);
        self.render()
    }

    fn end(&mut self, deferred_effects: &[DialogueEffectRequest]) -> Result<(), CliError> {
        self.state.deferred_queue = deferred_effects
            .iter()
            .map(|effect| TuiDeferredEffectRow {
                id: effect.id.as_str().to_owned(),
                function: effect.function.clone(),
                args: format_effect_arguments(&effect.args),
            })
            .collect();
        self.state.deferred_queue_state = Some(TuiDeferredQueueState::Ready);
        self.state.prompt = TuiPrompt::Finished {
            interaction: finished_interaction(),
        };
        self.push(TuiTranscriptKind::End, None, String::new())?;
        if !deferred_effects.is_empty() {
            self.state.transcript.push(TuiTranscriptEntry {
                kind: TuiTranscriptKind::Deferred,
                id: None,
                text: self.messages.text(MsgId::TuiTranscriptDeferredEffects),
            });
            for effect in deferred_effects {
                self.state.transcript.push(TuiTranscriptEntry {
                    kind: TuiTranscriptKind::Deferred,
                    id: Some(effect.id.as_str().to_owned()),
                    text: self.messages.format(
                        MsgId::TuiTranscriptDeferredEffectText,
                        [
                            ("function", effect.function.clone()),
                            ("args", format_effect_arguments(&effect.args)),
                        ],
                    ),
                });
            }
            self.render()?;
        }
        self.state.status = self.messages.text(MsgId::TuiFinished);
        self.wait_for_exit()
    }

    fn invalid_input(&mut self, message: String) -> Result<(), CliError> {
        self.state.status = self
            .messages
            .format(MsgId::PlayInvalidInput, [("message", message)]);
        self.render()
    }
}
