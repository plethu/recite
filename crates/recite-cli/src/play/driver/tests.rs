use std::{collections::VecDeque, path::Path};

use recite_compiler::{CompileInput, compile_inputs};
use recite_core::{ChoiceId, CompiledDialogue};
use recite_runtime::{
    ConditionQuery, ConditionValue, DialogueChoice, DialogueEffectRequest, DialogueLine,
};

use crate::fs::compile_options;
use crate::i18n::MsgId;

use super::*;

#[derive(Debug, Eq, PartialEq)]
enum UiEvent {
    Effect(String),
    Acknowledged(String),
    Queue {
        status: DeferredQueueStatus,
        functions: Vec<String>,
    },
    End(Vec<String>),
}

#[derive(Default)]
struct RecordingUi {
    events: Vec<UiEvent>,
    selections: VecDeque<ChoiceSelection>,
    invalid_inputs: Vec<String>,
    condition_false: bool,
    adapter_invalid_once: bool,
}

impl PlayUiAdapter for RecordingUi {
    fn message(
        &self,
        id: MsgId,
        _args: impl IntoIterator<Item = (&'static str, String)>,
    ) -> String {
        id.key().to_owned()
    }

    fn start(&mut self, _asset: &CompiledDialogue, _block: &str) -> Result<(), CliError> {
        Ok(())
    }

    fn line(&mut self, _line: &DialogueLine) -> Result<(), CliError> {
        Ok(())
    }

    fn choice(
        &mut self,
        _line: Option<&DialogueLine>,
        choices: &[DialogueChoice],
    ) -> Result<ChoiceSelection, CliError> {
        if self.adapter_invalid_once {
            self.adapter_invalid_once = false;
            return Err(CliError::PlayInvalidInput(
                "adapter rejected input".to_owned(),
            ));
        }
        Ok(self.selections.pop_front().unwrap_or_else(|| {
            ChoiceSelection::Id(
                choices
                    .first()
                    .expect("choice exists")
                    .id
                    .as_str()
                    .to_owned(),
            )
        }))
    }

    fn selected_choice(&mut self, _choice_id: &ChoiceId) -> Result<(), CliError> {
        Ok(())
    }

    fn condition(&mut self, _query: ConditionQuery<'_>) -> Result<ConditionValue, CliError> {
        Ok(ConditionValue::Bool(!self.condition_false))
    }

    fn effect(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        self.events.push(UiEvent::Effect(effect.function.clone()));
        Ok(())
    }

    fn acknowledge(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        self.events
            .push(UiEvent::Acknowledged(effect.function.clone()));
        Ok(())
    }

    fn deferred_queue(
        &mut self,
        effects: &[DialogueEffectRequest],
        status: DeferredQueueStatus,
    ) -> Result<(), CliError> {
        self.events.push(UiEvent::Queue {
            status,
            functions: effect_functions(effects),
        });
        Ok(())
    }

    fn end(&mut self, deferred_effects: &[DialogueEffectRequest]) -> Result<(), CliError> {
        self.events
            .push(UiEvent::End(effect_functions(deferred_effects)));
        Ok(())
    }

    fn invalid_input(&mut self, message: String) -> Result<(), CliError> {
        self.invalid_inputs.push(message);
        Ok(())
    }
}

#[test]
fn play_driver_recovers_from_adapter_reported_invalid_choice_without_nested_borrow() {
    let asset = asset(concat!(
        ":: start default\n",
        "> intro@18c570b9af4d973ba876\n",
        "  Choose.\n",
        "  ? go@c491f4cbe1944ebc5bc5\n",
        "    Go.\n",
        "    -> END\n",
    ));
    let mut ui = RecordingUi {
        selections: VecDeque::from([ChoiceSelection::Id("c491f4cbe1944ebc5bc5".to_owned())]),
        ..RecordingUi::default()
    };

    ui.adapter_invalid_once = true;
    PlayDriver::new(&asset, "start")
        .run(&mut ui)
        .expect("play succeeds after invalid adapter input");

    assert_eq!(ui.invalid_inputs, vec!["adapter rejected input"]);
}

#[test]
fn play_driver_reprompts_typed_unavailable_choice_without_nested_borrow() {
    let asset = asset(concat!(
        ":: start default\n",
        "> intro@18c570b9af4d973ba876\n",
        "  Choose.\n",
        "  ? locked@c491f4cbe1944ebc5bc5 requires=(trusts(player))\n",
        "    Locked.\n",
        "    -> END\n",
        "  ? open@d491f4cbe1944ebc5bc5\n",
        "    Open.\n",
        "    -> END\n",
    ));
    let mut ui = RecordingUi {
        selections: VecDeque::from([
            ChoiceSelection::Id("c491f4cbe1944ebc5bc5".to_owned()),
            ChoiceSelection::Id("d491f4cbe1944ebc5bc5".to_owned()),
        ]),
        condition_false: true,
        ..RecordingUi::default()
    };

    PlayDriver::new(&asset, "start")
        .run(&mut ui)
        .expect("play succeeds after unavailable typed choice");

    assert_eq!(ui.invalid_inputs.len(), 1);
    assert_eq!(
        ui.invalid_inputs[0],
        MsgId::PlayErrorChoiceUnavailable.key()
    );
}

#[test]
fn deferred_queue_updates_when_effects_are_scheduled_before_later_events() {
    let asset = asset(concat!(
        ":: start default\n",
        "> intro@18c570b9af4d973ba876\n",
        "  Choose.\n",
        "  ? go@c491f4cbe1944ebc5bc5\n",
        "    Go.\n",
        "    -> branch\n",
        ":: branch\n",
        "! deferred first_deferred()\n",
        "! immediate notify_game()\n",
        "! deferred second_deferred()\n",
        "! blocking grant_item(map)\n",
        "! deferred final_deferred()\n",
        "-> END\n",
    ));
    let mut ui = RecordingUi::default();

    PlayDriver::new(&asset, "start")
        .run(&mut ui)
        .expect("play succeeds");

    assert_eq!(
        ui.events,
        vec![
            UiEvent::Queue {
                status: DeferredQueueStatus::Scheduled,
                functions: vec!["first_deferred".to_owned()],
            },
            UiEvent::Effect("notify_game".to_owned()),
            UiEvent::Queue {
                status: DeferredQueueStatus::Scheduled,
                functions: vec!["first_deferred".to_owned(), "second_deferred".to_owned()],
            },
            UiEvent::Effect("grant_item".to_owned()),
            UiEvent::Acknowledged("grant_item".to_owned()),
            UiEvent::Queue {
                status: DeferredQueueStatus::Scheduled,
                functions: vec![
                    "first_deferred".to_owned(),
                    "second_deferred".to_owned(),
                    "final_deferred".to_owned(),
                ],
            },
            UiEvent::Queue {
                status: DeferredQueueStatus::Ready,
                functions: vec![
                    "first_deferred".to_owned(),
                    "second_deferred".to_owned(),
                    "final_deferred".to_owned(),
                ],
            },
            UiEvent::End(vec![
                "first_deferred".to_owned(),
                "second_deferred".to_owned(),
                "final_deferred".to_owned(),
            ]),
        ]
    );
}

fn asset(source: &str) -> CompiledDialogue {
    let report = compile_inputs(
        vec![CompileInput::new("test.recite", source)],
        compile_options(Path::new("test.recitec"), None).expect("options"),
    )
    .expect("compiles");
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    report.asset.expect("asset").dialogue
}

fn effect_functions(effects: &[DialogueEffectRequest]) -> Vec<String> {
    effects
        .iter()
        .map(|effect| effect.function.clone())
        .collect()
}
