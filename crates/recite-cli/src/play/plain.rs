use std::io::{self, Read, Write};

use recite_core::{ChoiceId, CompiledDialogue};
use recite_runtime::{ConditionQuery, DialogueChoice, DialogueEffectRequest, DialogueLine};

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
) -> Result<(), CliError> {
    let mut stdin = io::stdin().lock();
    let mut ui = PlainPlayUi::new(&mut stdin, stdout, messages);
    PlayDriver::new(asset, block).run(&mut ui)
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
            let availability = if choice.is_available {
                String::new()
            } else {
                self.messages.text(MsgId::PlayChoiceUnavailableSuffix)
            };
            writeln!(
                self.output,
                "{}",
                self.messages.format(
                    MsgId::PlayChoiceRow,
                    [
                        ("index", (index + 1).to_string()),
                        ("id", choice.id.as_str().to_owned()),
                        ("text", choice.text.clone()),
                        ("availability", availability),
                    ],
                )
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

    fn condition(&mut self, query: ConditionQuery<'_>) -> Result<bool, CliError> {
        let query = condition_query_text(query);
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
                    return Ok(true);
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
                    return Ok(false);
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
mod tests {
    use super::*;
    use std::path::Path;

    use recite_compiler::{CompileInput, compile_inputs};

    use crate::fs::compile_options;

    fn asset(source: &str) -> CompiledDialogue {
        let report = compile_inputs(
            vec![CompileInput::new("test.recite", source)],
            compile_options(Path::new("test.recitec"), None).expect("options"),
        )
        .expect("compiles");
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        report.asset.expect("asset").dialogue
    }

    fn run_plain(asset: &CompiledDialogue, input: &str) -> Result<String, CliError> {
        let mut input = input.as_bytes();
        let mut output = Vec::new();
        let messages = Messages::load(&crate::i18n::UiLocale::default()).expect("messages");
        let mut ui = PlainPlayUi::new(&mut input, &mut output, &messages);
        PlayDriver::new(asset, "start").run(&mut ui)?;
        Ok(String::from_utf8(output).expect("utf8"))
    }

    #[test]
    fn plain_play_selects_choice_by_index_answers_condition_and_acknowledges_blocking_effect() {
        let asset = asset(concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? help if trusts(player)\n",
            "    Help.\n",
            "    -> help\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> END\n",
            ":: help\n",
            "! blocking grant_item(map)\n",
            "> helped\n",
            "  Helped.\n",
            "! deferred finish(help)\n",
            "-> END\n",
        ));

        let output = run_plain(&asset, "y\n1\nack\n").expect("play succeeds");

        assert!(output.contains("condition trusts(player) = true"));
        assert!(output.contains("selected choice help"));
        assert!(output.contains("effect blocking"));
        assert!(output.contains("acknowledged effect"));
        assert!(output.contains("line helped: Helped."));
        assert!(output.contains("deferred effects:"));
    }

    #[test]
    fn plain_play_selects_choice_by_id() {
        let asset = asset(concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? help\n",
            "    Help.\n",
            "    -> help\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> END\n",
            ":: help\n",
            "> helped\n",
            "  Helped.\n",
            "-> END\n",
        ));

        let output = run_plain(&asset, "help\n").expect("play succeeds");

        assert!(output.contains("selected choice help"));
        assert!(output.contains("line helped: Helped."));
    }

    #[test]
    fn plain_play_can_select_numeric_choice_id() {
        let asset = asset(concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? skip\n",
            "    Skip.\n",
            "    -> skip\n",
            "  ? 2\n",
            "    Numeric.\n",
            "    -> numeric\n",
            ":: skip\n",
            "> skipped\n",
            "  Skipped.\n",
            "-> END\n",
            ":: numeric\n",
            "> numeric_line\n",
            "  Numeric ID selected.\n",
            "-> END\n",
        ));

        let output = run_plain(&asset, "2\n").expect("play succeeds");

        assert!(output.contains("selected choice 2"));
        assert!(output.contains("line numeric_line: Numeric ID selected."));
        assert!(!output.contains("selected choice skip"));
    }

    #[test]
    fn plain_play_reprompts_after_invalid_choice_and_condition_input() {
        let asset = asset(concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? help if trusts(player)\n",
            "    Help.\n",
            "    -> END\n",
        ));

        let output = run_plain(&asset, "maybe\ny\n\nbad id\n99\n1\n").expect("play succeeds");

        assert!(output.contains("invalid input: enter y or n"));
        assert!(output.contains("invalid input: choice selection cannot be empty"));
        assert!(output.contains("invalid input: choice ID `bad id` is not available here"));
        assert!(output.contains("invalid input: choice index 99 is out of range"));
        assert!(output.contains("selected choice help"));
    }

    #[test]
    fn plain_play_reprompts_for_unavailable_choice_without_recording_selection() {
        let asset = asset(concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? help if trusts(player)\n",
            "    Help.\n",
            "    -> help\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> leave\n",
            ":: help\n",
            "> helped\n",
            "  Helped.\n",
            "-> END\n",
            ":: leave\n",
            "> left\n",
            "  Left.\n",
            "-> END\n",
        ));

        let output = run_plain(&asset, "n\n1\nleave\n").expect("play succeeds");

        assert!(output.contains("condition trusts(player) = false"));
        assert!(output.contains("invalid input: choice `help` is unavailable"));
        assert!(!output.contains("selected choice help"));
        assert!(output.contains("selected choice leave"));
        assert!(output.contains("line left: Left."));
    }

    #[test]
    fn plain_play_reports_eof() {
        let asset = asset(concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? help\n",
            "    Help.\n",
            "    -> END\n",
        ));

        let error = run_plain(&asset, "").expect_err("eof fails");

        assert!(error.to_string().contains("reached EOF"));
    }

    #[test]
    fn plain_play_reports_condition_prompt_eof_as_cli_error() {
        let asset = asset(concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? help if trusts(player)\n",
            "    Help.\n",
            "    -> END\n",
        ));

        let error = run_plain(&asset, "").expect_err("eof fails");

        assert!(matches!(
            error,
            CliError::PlayEof {
                field: "condition answer"
            }
        ));
    }

    #[test]
    fn plain_play_reports_post_choice_condition_eof_as_cli_error() {
        let asset = asset(concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? help\n",
            "    Help.\n",
            "    -> help\n",
            ":: help\n",
            ":if trusts(player)\n",
            "  > helped\n",
            "    Helped.\n",
            "-> END\n",
        ));

        let error = run_plain(&asset, "help\n").expect_err("eof fails");

        assert!(matches!(
            error,
            CliError::PlayEof {
                field: "condition answer"
            }
        ));
    }
}
