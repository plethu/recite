use std::io::{Read, Write};

use recite_core::{ChoiceId, CompiledDialogue, EffectId};
use recite_runtime::{
    ConditionAnswer, DialogueEffectRequest, DialogueLine, PreviewConditionRequest,
    PreviewConditionResult, PreviewPrompt,
};

use crate::error::CliError;
use crate::i18n::{Messages, MsgId};

use super::preview::PreviewPlayUi;

pub(super) struct PlainPlayUi<'a, R: ?Sized, W: ?Sized> {
    pub(super) input: &'a mut R,
    pub(super) output: &'a mut W,
    pub(super) messages: &'a Messages,
}

impl<'a, R: ?Sized, W: Write + ?Sized> PlainPlayUi<'a, R, W> {
    pub(super) fn new(input: &'a mut R, output: &'a mut W, messages: &'a Messages) -> Self {
        Self {
            input,
            output,
            messages,
        }
    }

    pub(super) fn invalid_input(&mut self, message: String) -> Result<(), CliError> {
        writeln!(
            self.output,
            "{}",
            self.messages
                .format(MsgId::PlayInvalidInput, [("message", message)])
        )?;
        Ok(())
    }
}

impl<R: Read + ?Sized, W: Write + ?Sized> PreviewPlayUi for PlainPlayUi<'_, R, W> {
    fn start(&mut self, asset: &CompiledDialogue, block: &str) -> Result<(), CliError> {
        self.write_start(asset, block)
    }

    fn line(&mut self, line: &DialogueLine) -> Result<(), CliError> {
        self.write_line(line)
    }

    fn choice(&mut self, prompt: &PreviewPrompt) -> Result<ChoiceId, CliError> {
        self.read_choice(prompt)
    }

    fn selected_choice(&mut self, choice_id: &ChoiceId) -> Result<(), CliError> {
        self.write_selected_choice(choice_id)
    }

    fn condition(
        &mut self,
        request: &PreviewConditionRequest,
    ) -> Result<ConditionAnswer, CliError> {
        self.read_condition(request)
    }

    fn condition_result(
        &mut self,
        request: &PreviewConditionRequest,
        result: &PreviewConditionResult,
    ) -> Result<(), CliError> {
        self.write_condition_result(request, result)
    }

    fn effect(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        self.write_effect(effect)
    }

    fn acknowledge(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        self.read_acknowledgement(effect)
    }

    fn acknowledged(&mut self, effect_id: &EffectId) -> Result<(), CliError> {
        self.write_acknowledged(effect_id)
    }

    fn end(&mut self, deferred_effects: &[DialogueEffectRequest]) -> Result<(), CliError> {
        self.write_end(deferred_effects)
    }
}
