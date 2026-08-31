use std::path::Path;

use crate::error::CliError;
use crate::i18n::{Messages, UiLocale};
use crate::play::plain_ui::PlainPlayUi;
use crate::play::preview::{PreviewPlayUi, run_preview};
use crate::tui::{Keymap, PromptMode, TextBuffer, TuiInteractionState};
use recite_compiler::{CompileInput, compile_inputs};
use recite_core::{ChoiceId, CompiledDialogue, EffectId};
use recite_runtime::{
    ConditionAnswer, ConditionExpectedType, ConditionValue, DialogueEffectRequest, DialogueLine,
    PreviewConditionRequest, PreviewConditionResult, PreviewPrompt,
};

use super::interaction::enum_condition_variant;
use super::preview::condition_prompt;
use super::state::TuiPrompt;

#[test]
fn condition_prompt_uses_expected_type_specific_state() {
    let boolean = condition_prompt(
        ConditionExpectedType::Bool,
        "trusts(mira)".to_owned(),
        Keymap::Standard,
    );
    assert_eq!(
        boolean,
        TuiPrompt::Condition {
            query: "trusts(mira)".to_owned(),
            selected: true,
            interaction: TuiInteractionState::new(PromptMode::Insert),
        }
    );

    let enumeration = condition_prompt(
        ConditionExpectedType::Enum,
        "memory_pressure(hazel, music_shop)".to_owned(),
        Keymap::Vim,
    );
    assert_eq!(
        enumeration,
        TuiPrompt::EnumCondition {
            query: "memory_pressure(hazel, music_shop)".to_owned(),
            interaction: TuiInteractionState::new(PromptMode::Normal),
            input: TextBuffer::default(),
        }
    );
}

#[test]
fn enum_condition_variant_trims_non_empty_input() {
    let messages = Messages::load(&UiLocale::default()).expect("messages");

    assert_eq!(
        enum_condition_variant("  high  ", &messages).expect("variant"),
        "high"
    );
}

#[test]
fn enum_condition_variant_rejects_empty_input() {
    let messages = Messages::load(&UiLocale::default()).expect("messages");

    assert_eq!(
        enum_condition_variant("  ", &messages)
            .expect_err("empty input")
            .to_string(),
        "invalid play input: enter an enum variant"
    );
}

#[derive(Default)]
struct HeadlessTui {
    events: Vec<String>,
}

impl PreviewPlayUi for HeadlessTui {
    fn start(&mut self, _asset: &CompiledDialogue, block: &str) -> Result<(), CliError> {
        self.events.push(format!("start:{block}"));
        Ok(())
    }

    fn line(&mut self, line: &DialogueLine) -> Result<(), CliError> {
        self.events.push(format!("line:{}", line.id.as_str()));
        Ok(())
    }

    fn choice(&mut self, prompt: &PreviewPrompt) -> Result<ChoiceId, CliError> {
        self.events.push("prompt".to_owned());
        prompt
            .choices()
            .iter()
            .find(|choice| choice.availability.is_available)
            .map(|choice| choice.id.clone())
            .ok_or_else(|| CliError::PlayInvalidInput("no available choice".to_owned()))
    }

    fn selected_choice(&mut self, choice_id: &ChoiceId) -> Result<(), CliError> {
        self.events.push(format!("choice:{}", choice_id.as_str()));
        Ok(())
    }

    fn condition(
        &mut self,
        request: &PreviewConditionRequest,
    ) -> Result<ConditionAnswer, CliError> {
        self.events
            .push(format!("condition-request:{}", request.query().function()));
        Ok(match request.query().expected_type() {
            ConditionExpectedType::Bool => ConditionAnswer::Value(ConditionValue::Bool(true)),
            ConditionExpectedType::Enum => {
                ConditionAnswer::Value(ConditionValue::EnumVariant("high".to_owned()))
            }
        })
    }

    fn condition_result(
        &mut self,
        _request: &PreviewConditionRequest,
        _result: &PreviewConditionResult,
    ) -> Result<(), CliError> {
        self.events.push("condition-result".to_owned());
        Ok(())
    }

    fn effect(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        self.events.push(format!("effect:{}", effect.mode));
        Ok(())
    }

    fn acknowledge(&mut self, _effect: &DialogueEffectRequest) -> Result<(), CliError> {
        self.events.push("ack-request".to_owned());
        Ok(())
    }

    fn acknowledged(&mut self, _effect_id: &EffectId) -> Result<(), CliError> {
        self.events.push("ack".to_owned());
        Ok(())
    }

    fn deferred_effect_scheduled(
        &mut self,
        effect: &DialogueEffectRequest,
    ) -> Result<(), CliError> {
        self.events.push(format!("deferred:{}", effect.function));
        Ok(())
    }

    fn invalid_input(&mut self, message: String) -> Result<(), CliError> {
        self.events.push(format!("invalid:{message}"));
        Ok(())
    }

    fn end(&mut self, _deferred_effects: &[DialogueEffectRequest]) -> Result<(), CliError> {
        self.events.push("end".to_owned());
        Ok(())
    }
}

fn asset(source: &str) -> CompiledDialogue {
    let report = compile_inputs(
        vec![CompileInput::new("test.recite", source)],
        crate::fs::compile_options(Path::new("test.recitec"), None).expect("options"),
    )
    .expect("compiles");
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    report.asset.expect("asset").dialogue
}

#[test]
fn tui_preview_and_plain_share_typed_event_progression() {
    let asset = asset(concat!(
        ":: start default\n",
        "> intro@18c570b9af4d973ba876\n",
        "  Choose.\n",
        "  ? go@c491f4cbe1944ebc5bc5 requires=(trusts(player))\n",
        "    Go.\n",
        "    -> branch\n",
        ":: branch\n",
        ":if trusts(player)\n",
        "  ! deferred save(slot)\n",
        "  ! immediate ping()\n",
        "  ! blocking grant(map)\n",
        "  > done@d491f4cbe1944ebc5bc5\n",
        "    Done.\n",
        "-> END\n",
    ));
    let mut tui = HeadlessTui::default();
    run_preview(&asset, "start", None, &mut tui).expect("TUI preview succeeds");
    assert_eq!(
        tui.events,
        vec![
            "start:start",
            "condition-request:trusts",
            "condition-result",
            "prompt",
            "condition-request:trusts",
            "choice:c491f4cbe1944ebc5bc5",
            "condition-result",
            "deferred:save",
            "effect:immediate",
            "effect:blocking",
            "ack-request",
            "ack",
            "line:d491f4cbe1944ebc5bc5",
            "end",
        ]
    );

    let messages = Messages::load(&UiLocale::default()).expect("messages");
    let mut input = "y\n1\ny\nack\n".as_bytes();
    let mut output = Vec::new();
    let mut plain = PlainPlayUi::new(&mut input, &mut output, &messages);
    run_preview(&asset, "start", None, &mut plain).expect("plain preview succeeds");
    let output = String::from_utf8(output).expect("UTF-8 output");
    assert!(output.contains("selected choice c491f4cbe1944ebc5bc5"));
    assert!(output.contains("effect immediate"));
    assert!(output.contains("effect blocking"));
    assert!(output.contains("acknowledged effect"));
    assert!(output.contains("line d491f4cbe1944ebc5bc5: Done."));
}
