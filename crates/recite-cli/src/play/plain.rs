use std::io::{self, Read, Write};

use recite_core::{ChoiceId, CompiledDialogue};
use recite_runtime::{
    ConditionExpectedType, ConditionQuery, ConditionValue, DialogueChoice, DialogueEffectRequest,
    DialogueLine,
};
use recite_ui::{UiArg, UiArgs};

use crate::dialogue_locale::DialogueTraversalPreview;
use crate::error::CliError;
use crate::i18n::{Messages, MsgId};
use crate::runtime_format::format_effect_arguments;

use super::driver::{ChoiceSelection, PlayDriver, PlayUiAdapter};
use super::format::condition_query_text;

pub(super) fn run_plain_stdio(
    asset: &CompiledDialogue,
    block: &str,
    stdout: &mut dyn Write,
    messages: &Messages,
    dialogue_preview: Option<DialogueTraversalPreview<'_>>,
) -> Result<(), CliError> {
    let mut stdin = io::stdin().lock();
    let mut ui = PlainPlayUi::new(&mut stdin, stdout, messages);
    let driver = PlayDriver::new(asset, block);
    match dialogue_preview {
        Some(preview) => driver.with_dialogue_preview(preview).run(&mut ui),
        None => driver.run(&mut ui),
    }
}

struct PlainPlayUi<'a, R: ?Sized, W: ?Sized> {
    input: &'a mut R,
    output: &'a mut W,
    messages: &'a Messages,
}

impl<'a, R: ?Sized, W: ?Sized> PlainPlayUi<'a, R, W> {
    fn new(input: &'a mut R, output: &'a mut W, messages: &'a Messages) -> Self {
        Self {
            input,
            output,
            messages,
        }
    }
}

impl<R: Read + ?Sized, W: Write + ?Sized> PlayUiAdapter for PlainPlayUi<'_, R, W> {
    fn message(&self, id: MsgId, args: impl IntoIterator<Item = (&'static str, String)>) -> String {
        self.messages.format(id, args)
    }

    fn message_typed(&self, id: MsgId, args: UiArgs) -> String {
        self.messages.format_args(id, &args)
    }

    fn start(&mut self, asset: &CompiledDialogue, block: &str) -> Result<(), CliError> {
        writeln!(
            self.output,
            "{}",
            self.messages.format(
                MsgId::PlayStart,
                [
                    ("asset", asset.header.asset_id.as_str().to_owned()),
                    ("block", block.to_owned()),
                ],
            )
        )?;
        Ok(())
    }

    fn line(&mut self, line: &DialogueLine) -> Result<(), CliError> {
        writeln!(
            self.output,
            "{}",
            self.messages.format(
                MsgId::PlayLine,
                [
                    ("id", line.id.as_str().to_owned()),
                    ("text", line.text.clone()),
                ],
            )
        )?;
        Ok(())
    }

    fn choice(
        &mut self,
        line: Option<&DialogueLine>,
        choices: &[DialogueChoice],
    ) -> Result<ChoiceSelection, CliError> {
        if let Some(line) = line {
            writeln!(
                self.output,
                "{}",
                self.messages.format(
                    MsgId::PlayPromptLine,
                    [
                        ("id", line.id.as_str().to_owned()),
                        ("text", line.text.clone()),
                    ],
                )
            )?;
        } else {
            writeln!(self.output, "{}", self.messages.text(MsgId::PlayPrompt))?;
        }
        for (index, choice) in choices.iter().enumerate() {
            let args = UiArgs::from([
                ("index".to_owned(), UiArg::from(index + 1)),
                ("id".to_owned(), UiArg::from(choice.id.as_str())),
                ("text".to_owned(), UiArg::from(choice.text.as_str())),
                (
                    "available".to_owned(),
                    UiArg::from(choice.availability.is_available),
                ),
            ]);
            writeln!(
                self.output,
                "{}",
                self.messages.format_args(MsgId::PlayChoiceRow, &args)
            )?;
        }
        write!(
            self.output,
            "{} ",
            self.messages.text(MsgId::PlayChoicePrompt)
        )?;
        self.output.flush()?;
        let input = read_line(self.input, "choice selection")?;
        ChoiceSelection::parse(input.trim(), self.messages)
    }

    fn condition(&mut self, query: ConditionQuery<'_>) -> Result<ConditionValue, CliError> {
        let expected_type = query.expected_type();
        let query = condition_query_text(query);
        if matches!(expected_type, ConditionExpectedType::Enum) {
            write!(
                self.output,
                "{} ",
                self.messages
                    .format(MsgId::PlayConditionPrompt, [("query", query.clone())])
            )?;
            self.output.flush()?;
            let input = read_line(self.input, "condition enum answer")?;
            let value = input.trim().to_owned();
            if value.is_empty() {
                return Err(CliError::PlayInvalidInput(
                    self.messages.text(MsgId::PlayErrorEnterEnumVariant),
                ));
            }
            writeln!(
                self.output,
                "{}",
                self.messages.format(
                    MsgId::PlayConditionResult,
                    [("query", query), ("result", value.clone())],
                )
            )?;
            return Ok(ConditionValue::EnumVariant(value));
        }

        loop {
            write!(
                self.output,
                "{} ",
                self.messages
                    .format(MsgId::PlayConditionPrompt, [("query", query.clone())])
            )?;
            self.output.flush()?;
            let input = read_line(self.input, "condition answer")?;
            match input.trim().to_ascii_lowercase().as_str() {
                "y" | "yes" | "true" | "1" => {
                    writeln!(
                        self.output,
                        "{}",
                        self.messages.format(
                            MsgId::PlayConditionResult,
                            [("query", query.clone()), ("result", "true".to_owned())],
                        )
                    )?;
                    return Ok(ConditionValue::Bool(true));
                }
                "n" | "no" | "false" | "0" => {
                    writeln!(
                        self.output,
                        "{}",
                        self.messages.format(
                            MsgId::PlayConditionResult,
                            [("query", query.clone()), ("result", "false".to_owned())],
                        )
                    )?;
                    return Ok(ConditionValue::Bool(false));
                }
                _ => self.invalid_input(self.messages.text(MsgId::PlayErrorEnterYOrN))?,
            }
        }
    }

    fn selected_choice(&mut self, choice_id: &ChoiceId) -> Result<(), CliError> {
        writeln!(
            self.output,
            "{}",
            self.messages.format(
                MsgId::PlaySelectedChoice,
                [("id", choice_id.as_str().to_owned())],
            )
        )?;
        Ok(())
    }

    fn effect(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        writeln!(
            self.output,
            "{}",
            self.messages.format(
                MsgId::PlayEffect,
                [
                    ("mode", effect.mode.to_string()),
                    ("id", effect.id.as_str().to_owned()),
                    ("function", effect.function.clone()),
                    ("args", format_effect_arguments(&effect.args)),
                ],
            )
        )?;
        Ok(())
    }

    fn acknowledge(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        loop {
            write!(
                self.output,
                "{} ",
                self.messages.format(
                    MsgId::PlayAckPrompt,
                    [("id", effect.id.as_str().to_owned())],
                )
            )?;
            self.output.flush()?;
            let input = read_line(self.input, "blocking effect acknowledgement")?;
            let input = input.trim();
            if input.is_empty() || input.eq_ignore_ascii_case("ack") {
                writeln!(
                    self.output,
                    "{}",
                    self.messages.format(
                        MsgId::PlayAckCompleted,
                        [("id", effect.id.as_str().to_owned())],
                    )
                )?;
                return Ok(());
            }
            self.invalid_input(self.messages.text(MsgId::PlayErrorPressEnterOrAck))?;
        }
    }

    fn end(&mut self, deferred_effects: &[DialogueEffectRequest]) -> Result<(), CliError> {
        writeln!(self.output, "{}", self.messages.text(MsgId::PlayEnd))?;
        if !deferred_effects.is_empty() {
            writeln!(
                self.output,
                "{}",
                self.messages.text(MsgId::PlayDeferredEffects)
            )?;
            for effect in deferred_effects {
                writeln!(
                    self.output,
                    "{}",
                    self.messages.format(
                        MsgId::PlayDeferredEffectRow,
                        [
                            ("function", effect.function.clone()),
                            ("args", format_effect_arguments(&effect.args)),
                        ],
                    )
                )?;
            }
        }
        Ok(())
    }

    fn invalid_input(&mut self, message: String) -> Result<(), CliError> {
        writeln!(
            self.output,
            "{}",
            self.messages
                .format(MsgId::PlayInvalidInput, [("message", message)])
        )?;
        Ok(())
    }
}

fn read_line<R: Read + ?Sized>(input: &mut R, field: &'static str) -> Result<String, CliError> {
    let mut byte = [0_u8; 1];
    let mut line = Vec::new();
    loop {
        match input.read(&mut byte) {
            Ok(0) if line.is_empty() => return Err(CliError::PlayEof { field }),
            Ok(0) => break,
            Ok(_) if byte[0] == b'\n' => break,
            Ok(_) => line.push(byte[0]),
            Err(error) => return Err(CliError::Io(error)),
        }
    }
    Ok(String::from_utf8_lossy(&line).into_owned())
}

#[cfg(test)]
mod tests;
