use recite_core::{ChoiceId, CompiledDialogue};
use recite_runtime::{
    ConditionQuery, ConditionValue, DialogueChoice, DialogueEffectRequest, DialogueLine,
};
use recite_ui::UiArgs;

use crate::error::CliError;
use crate::i18n::{Messages, MsgId};

use super::driver::DeferredQueueStatus;

pub(super) trait PlayUiAdapter {
    fn message(&self, id: MsgId, args: impl IntoIterator<Item = (&'static str, String)>) -> String;
    fn message_typed(&self, id: MsgId, args: UiArgs) -> String {
        self.message(
            id,
            args.into_iter().map(|(name, value)| {
                let name: &'static str = Box::leak(name.into_boxed_str());
                (name, value.to_string())
            }),
        )
    }
    fn start(&mut self, asset: &CompiledDialogue, block: &str) -> Result<(), CliError>;
    fn line(&mut self, line: &DialogueLine) -> Result<(), CliError>;
    fn choice(
        &mut self,
        line: Option<&DialogueLine>,
        choices: &[DialogueChoice],
    ) -> Result<ChoiceSelection, CliError>;
    fn selected_choice(&mut self, choice_id: &ChoiceId) -> Result<(), CliError>;
    fn condition(&mut self, query: ConditionQuery<'_>) -> Result<ConditionValue, CliError>;
    fn effect(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError>;
    fn acknowledge(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError>;
    fn deferred_queue(
        &mut self,
        _effects: &[DialogueEffectRequest],
        _status: DeferredQueueStatus,
    ) -> Result<(), CliError> {
        Ok(())
    }
    fn end(&mut self, deferred_effects: &[DialogueEffectRequest]) -> Result<(), CliError>;
    fn invalid_input(&mut self, message: String) -> Result<(), CliError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ChoiceSelection {
    Index(usize),
    Id(String),
}

impl ChoiceSelection {
    pub(super) fn parse(input: &str, messages: &Messages) -> Result<Self, CliError> {
        if input.is_empty() {
            return Err(CliError::PlayInvalidInput(
                messages.text(MsgId::PlayErrorEmptyChoice),
            ));
        }
        if let Ok(index) = input.parse::<usize>() {
            return Ok(Self::Index(index));
        }
        Ok(Self::Id(input.to_owned()))
    }
}
