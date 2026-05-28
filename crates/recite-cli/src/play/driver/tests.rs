use std::path::Path;

use recite_compiler::{CompileInput, compile_inputs};

use crate::fs::compile_options;

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
        Ok(ChoiceSelection::Id(
            choices
                .first()
                .expect("choice exists")
                .id
                .as_str()
                .to_owned(),
        ))
    }

    fn selected_choice(&mut self, _choice_id: &ChoiceId) -> Result<(), CliError> {
        Ok(())
    }

    fn condition(&mut self, _query: ConditionQuery<'_>) -> Result<ConditionValue, CliError> {
        Ok(ConditionValue::Bool(true))
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

    fn invalid_input(&mut self, _message: String) -> Result<(), CliError> {
        Ok(())
    }
}

#[test]
fn deferred_queue_updates_when_effects_are_scheduled_before_later_events() {
    let asset = asset(concat!(
        ":: start default\n",
        "> intro\n",
        "  Choose.\n",
        "  ? go\n",
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
