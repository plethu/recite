use std::io::{Read, Write};

use recite_core::ChoiceId;
use recite_runtime::{DialogueChoice, PreviewPrompt};

use crate::error::CliError;
use crate::i18n::{Messages, MsgId};

use super::choice_selection::ChoiceSelection;
use super::plain_input::read_line;
use super::plain_ui::PlainPlayUi;

impl<R: Read + ?Sized, W: Write + ?Sized> PlainPlayUi<'_, R, W> {
    pub(super) fn read_choice(&mut self, prompt: &PreviewPrompt) -> Result<ChoiceId, CliError> {
        self.write_prompt(prompt)?;
        loop {
            let input = read_line(self.input, "choice selection")?;
            let selection = match ChoiceSelection::parse(input.trim(), self.messages) {
                Ok(selection) => selection,
                Err(CliError::PlayInvalidInput(message)) => {
                    self.invalid_input(message)?;
                    self.write_prompt(prompt)?;
                    continue;
                }
                Err(error) => return Err(error),
            };
            match self.resolve_choice(selection, prompt) {
                Ok(choice) => return Ok(choice),
                Err(message) => self.invalid_input(message)?,
            }
            self.write_prompt(prompt)?;
        }
    }

    fn write_prompt(&mut self, prompt: &PreviewPrompt) -> Result<(), CliError> {
        if let Some(line) = prompt.line() {
            writeln!(
                self.output,
                "{}",
                self.messages.format(
                    MsgId::PlayPromptLine,
                    [
                        ("id", line.id.as_str().to_owned()),
                        ("text", line.text.clone()),
                    ]
                )
            )?;
        } else {
            writeln!(self.output, "{}", self.messages.text(MsgId::PlayPrompt))?;
        }
        for (index, choice) in prompt.choices().iter().enumerate() {
            let args = recite_ui::UiArgs::from([
                ("index".to_owned(), recite_ui::UiArg::from(index + 1)),
                ("id".to_owned(), recite_ui::UiArg::from(choice.id.as_str())),
                (
                    "text".to_owned(),
                    recite_ui::UiArg::from(choice.text.as_str()),
                ),
                (
                    "available".to_owned(),
                    recite_ui::UiArg::from(choice.availability.is_available),
                ),
            ]);
            writeln!(
                self.output,
                "{}",
                self.messages.format_args(MsgId::PlayChoiceRow, &args)
            )?;
        }
        self.write_choice_prompt()
    }

    fn write_choice_prompt(&mut self) -> Result<(), CliError> {
        write!(
            self.output,
            "{} ",
            self.messages.text(MsgId::PlayChoicePrompt)
        )?;
        self.output.flush()?;
        Ok(())
    }

    fn resolve_choice(
        &self,
        selection: ChoiceSelection,
        prompt: &PreviewPrompt,
    ) -> Result<ChoiceId, String> {
        match selection {
            ChoiceSelection::Index(index) => {
                let numeric_id = index.to_string();
                let choice = if let Some(choice) = prompt
                    .choices()
                    .iter()
                    .find(|choice| choice.id.as_str() == numeric_id)
                {
                    choice
                } else if index == 0 || index > prompt.choices().len() {
                    return Err(self.messages.format(
                        MsgId::PlayErrorChoiceIndexOutOfRange,
                        recite_ui::UiArgs::from([
                            ("index".to_owned(), recite_ui::UiArg::from(index)),
                            (
                                "count".to_owned(),
                                recite_ui::UiArg::from(prompt.choices().len()),
                            ),
                        ]),
                    ));
                } else {
                    &prompt.choices()[index - 1]
                };
                if choice.availability.is_available {
                    Ok(choice.id.clone())
                } else {
                    Err(unavailable_choice_message(self.messages, choice))
                }
            }
            ChoiceSelection::Id(id) => {
                let choice_id = ChoiceId::new(id.clone()).map_err(|error| {
                    self.messages.format(
                        MsgId::PlayErrorChoiceIdInvalid,
                        [("id", id.clone()), ("error", error.to_string())],
                    )
                })?;
                let Some(choice) = prompt
                    .choices()
                    .iter()
                    .find(|choice| choice.id == choice_id)
                else {
                    return Err(self
                        .messages
                        .format(MsgId::PlayErrorChoiceIdUnavailable, [("id", id)]));
                };
                if choice.availability.is_available {
                    Ok(choice_id)
                } else {
                    Err(unavailable_choice_message(self.messages, choice))
                }
            }
        }
    }
}

fn unavailable_choice_message(messages: &Messages, choice: &DialogueChoice) -> String {
    match choice
        .availability
        .primary_reason
        .as_ref()
        .map(|reason| reason.source_text.as_str())
    {
        Some(reason) if !reason.is_empty() => messages.format(
            MsgId::PlayErrorChoiceUnavailableReason,
            [
                ("id", choice.id.as_str().to_owned()),
                ("reason", reason.to_owned()),
            ],
        ),
        _ => messages.format(
            MsgId::PlayErrorChoiceUnavailable,
            [("id", choice.id.as_str().to_owned())],
        ),
    }
}
