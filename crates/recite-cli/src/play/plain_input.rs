use std::io::{Read, Write};

use recite_runtime::{
    ConditionAnswer, ConditionExpectedType, ConditionValue, PreviewConditionArgument,
    PreviewConditionRequest,
};

use crate::error::CliError;
use crate::i18n::MsgId;
use crate::runtime_format::{RuntimeDisplayArgument, format_condition_query};

use super::plain_ui::PlainPlayUi;

impl<R: Read + ?Sized, W: Write + ?Sized> PlainPlayUi<'_, R, W> {
    pub(super) fn read_condition(
        &mut self,
        request: &PreviewConditionRequest,
    ) -> Result<ConditionAnswer, CliError> {
        let query = condition_query_text(request)?;
        match request.query().expected_type() {
            ConditionExpectedType::Enum => loop {
                self.write_condition_prompt(&query)?;
                let value = read_line(self.input, "condition enum answer")?
                    .trim()
                    .to_owned();
                if value.is_empty() {
                    self.invalid_input(self.messages.text(MsgId::PlayErrorEnterEnumVariant))?;
                    continue;
                }
                return Ok(ConditionAnswer::Value(ConditionValue::EnumVariant(value)));
            },
            ConditionExpectedType::Bool => loop {
                self.write_condition_prompt(&query)?;
                match read_line(self.input, "condition answer")?
                    .trim()
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "y" | "yes" | "true" | "1" => return Ok(bool_answer(true)),
                    "n" | "no" | "false" | "0" => return Ok(bool_answer(false)),
                    _ => self.invalid_input(self.messages.text(MsgId::PlayErrorEnterYOrN))?,
                }
            },
        }
    }

    pub(super) fn read_acknowledgement(
        &mut self,
        effect: &recite_runtime::DialogueEffectRequest,
    ) -> Result<(), CliError> {
        loop {
            write!(
                self.output,
                "{} ",
                self.messages.format(
                    MsgId::PlayAckPrompt,
                    [("id", effect.id.as_str().to_owned())]
                )
            )?;
            self.output.flush()?;
            let input = read_line(self.input, "blocking effect acknowledgement")?;
            if input.trim().is_empty() || input.trim().eq_ignore_ascii_case("ack") {
                return Ok(());
            }
            self.invalid_input(self.messages.text(MsgId::PlayErrorPressEnterOrAck))?;
        }
    }

    fn write_condition_prompt(&mut self, query: &str) -> Result<(), CliError> {
        write!(
            self.output,
            "{} ",
            self.messages
                .format(MsgId::PlayConditionPrompt, [("query", query.to_owned())])
        )?;
        self.output.flush()?;
        Ok(())
    }
}

pub(super) fn condition_query_text(request: &PreviewConditionRequest) -> Result<String, CliError> {
    let arguments = request
        .query()
        .arguments()
        .iter()
        .map(|argument| match argument {
            PreviewConditionArgument::Identifier(value) => {
                Ok(RuntimeDisplayArgument::Identifier(value))
            }
            PreviewConditionArgument::String(value) => Ok(RuntimeDisplayArgument::String(value)),
            PreviewConditionArgument::Integer(value) => Ok(RuntimeDisplayArgument::Integer(*value)),
            PreviewConditionArgument::Float(value) => Ok(RuntimeDisplayArgument::Float(*value)),
            PreviewConditionArgument::Boolean(value) => Ok(RuntimeDisplayArgument::Boolean(*value)),
            _ => Err(CliError::Preview(recite_runtime::PreviewError::Runtime(
                recite_runtime::DialogueError::MalformedCompiledAsset {
                    reason: "unsupported condition argument".to_owned(),
                },
            ))),
        });
    Ok(format_condition_query(
        request.query().function(),
        arguments.collect::<Result<Vec<_>, _>>()?,
    ))
}

fn bool_answer(value: bool) -> ConditionAnswer {
    ConditionAnswer::Value(ConditionValue::Bool(value))
}

pub(super) fn read_line<R: Read + ?Sized>(
    input: &mut R,
    field: &'static str,
) -> Result<String, CliError> {
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
