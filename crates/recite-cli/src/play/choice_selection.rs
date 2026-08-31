use crate::i18n::{Messages, MsgId};

use crate::error::CliError;

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
