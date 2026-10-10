use proptest::prelude::*;
use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    ChoiceId,
    compiled::{
        CompiledAssetId, CompiledDialogue, CompilerVersion, SchemaFingerprint, SourceMapId,
    },
};
use recite_runtime::{
    ConditionEvaluationError, ConditionQuery, ConditionValue, DialogueError, DialogueEvent,
    DialogueSession, choose, next,
    snapshot::{decode_session_messagepack, encode_session_messagepack, snapshot_session},
    start_scene,
};

#[path = "session_properties/malformed.rs"]
mod malformed;
#[path = "session_properties/reasons.rs"]
mod reasons;

#[derive(Clone, Copy, Debug)]
enum Action {
    Next,
    Continue,
    Quit,
    UnknownChoice,
}

#[derive(Debug, PartialEq)]
enum Observation {
    Prompt(Vec<(String, bool)>),
    End,
    PromptPending,
    NoPrompt,
    UnknownChoice,
    UnavailableChoice,
    AlreadyEnded,
}

#[derive(Clone, Copy, Debug)]
enum Phase {
    Running,
    Prompt(usize),
    Ended,
}

struct Model {
    phase: Phase,
    history: Vec<String>,
}

impl Model {
    fn apply(&mut self, action: Action, gates: &[bool]) -> Observation {
        let stage = match self.phase {
            Phase::Running => {
                if matches!(action, Action::Next) {
                    self.phase = Phase::Prompt(0);
                    return prompt(0, gates);
                }
                return Observation::NoPrompt;
            }
            Phase::Ended => {
                return if matches!(action, Action::Next) {
                    Observation::AlreadyEnded
                } else {
                    Observation::NoPrompt
                };
            }
            Phase::Prompt(stage) => stage,
        };
        match action {
            Action::Next => Observation::PromptPending,
            Action::UnknownChoice => Observation::UnknownChoice,
            Action::Continue if !gates[stage] => Observation::UnavailableChoice,
            Action::Continue | Action::Quit => {
                self.history
                    .push(choice_id(stage, matches!(action, Action::Quit)));
                if matches!(action, Action::Quit) || stage + 1 == gates.len() {
                    self.phase = Phase::Ended;
                    Observation::End
                } else {
                    self.phase = Phase::Prompt(stage + 1);
                    prompt(stage + 1, gates)
                }
            }
        }
    }
}

fn choice_id(stage: usize, quit: bool) -> String {
    format!("{:020x}", stage * 3 + if quit { 3 } else { 2 })
}

fn prompt(stage: usize, gates: &[bool]) -> Observation {
    Observation::Prompt(vec![
        (choice_id(stage, false), gates[stage]),
        (choice_id(stage, true), true),
    ])
}

fn fixture(stages: usize) -> Result<CompiledDialogue, TestCaseError> {
    let mut source = String::new();
    for stage in 0..stages {
        let default = if stage == 0 { " default" } else { "" };
        let target = if stage + 1 == stages {
            "END".to_owned()
        } else {
            format!("stage_{}", stage + 1)
        };
        source.push_str(&format!(
            ":: stage_{stage}{default}\n> prompt_{stage}@{:020x}\n  Next?\n  ? continue_{stage}@{} requires=(gate_{stage}())\n    Continue.\n    -> {target}\n  ? quit_{stage}@{}\n    Quit.\n    -> END\n",
            stage * 3 + 1, choice_id(stage, false), choice_id(stage, true)
        ));
    }
    let options = CompileOptions::new(
        CompilerVersion::new("0.0.1").map_err(failure)?,
        CompiledAssetId::new("properties.recitec").map_err(failure)?,
        SourceMapId::new("properties.recitec.map").map_err(failure)?,
        SchemaFingerprint::NoSchema,
    );
    let report = compile_inputs([CompileInput::new("properties.recite", source)], options)
        .map_err(failure)?;
    prop_assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    report
        .asset
        .map(|asset| asset.dialogue)
        .ok_or_else(|| TestCaseError::fail("valid generated source must compile"))
}

fn failure(error: impl std::fmt::Display) -> TestCaseError {
    TestCaseError::fail(error.to_string())
}

fn observe(result: &Result<DialogueEvent, DialogueError>) -> Result<Observation, TestCaseError> {
    Ok(match result {
        Ok(DialogueEvent::Prompt { choices, .. }) => Observation::Prompt(
            choices
                .iter()
                .map(|choice| {
                    (
                        choice.id.as_str().to_owned(),
                        choice.availability.is_available,
                    )
                })
                .collect(),
        ),
        Ok(DialogueEvent::End { deferred_effects }) if deferred_effects.is_empty() => {
            Observation::End
        }
        Err(DialogueError::PromptPending { .. }) => Observation::PromptPending,
        Err(DialogueError::NoPromptPending { .. }) => Observation::NoPrompt,
        Err(DialogueError::InvalidChoice { .. }) => Observation::UnknownChoice,
        Err(DialogueError::UnavailableChoice { availability, .. })
            if !availability.is_available =>
        {
            Observation::UnavailableChoice
        }
        Err(DialogueError::SessionEnded) => Observation::AlreadyEnded,
        other => {
            return Err(TestCaseError::fail(format!(
                "unexpected generated event: {other:?}"
            )));
        }
    })
}

fn apply(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    action: Action,
    stage: usize,
    gates: &[bool],
) -> Result<Result<DialogueEvent, DialogueError>, TestCaseError> {
    let context = |query: ConditionQuery<'_>| {
        let stage = query
            .function()
            .strip_prefix("gate_")
            .and_then(|number| number.parse::<usize>().ok());
        stage
            .and_then(|index| gates.get(index))
            .copied()
            .map(ConditionValue::Bool)
            .ok_or_else(|| ConditionEvaluationError::new("unknown generated gate"))
    };
    if matches!(action, Action::Next) {
        return Ok(next(asset, session, &context));
    }
    let id = match action {
        Action::Continue => choice_id(stage, false),
        Action::Quit => choice_id(stage, true),
        Action::UnknownChoice => "ffffffffffffffffffff".to_owned(),
        Action::Next => unreachable!(),
    };
    Ok(choose(
        asset,
        session,
        ChoiceId::new(id).map_err(failure)?,
        &context,
    ))
}

proptest! {
    #[test]
    fn checkpointed_actions_follow_an_independent_choice_model(
        gates in prop::collection::vec(any::<bool>(), 1..7),
        actions in prop::collection::vec((0u8..4, any::<bool>()), 1..48),
    ) {
        let asset = fixture(gates.len())?;
        let mut live = start_scene(&asset, None).map_err(failure)?;
        let mut restored = live.clone();
        let mut model = Model { phase: Phase::Running, history: Vec::new() };
        // Every trace reaches a live prompt before the generated suffix explores transitions.
        let actions = [(3, true), (0, true), (3, true), (1, true)].into_iter().chain(actions);
        for (action, checkpoint) in actions {
            let action = match action {
                0 => Action::Next,
                1 => Action::Continue,
                2 => Action::Quit,
                _ => Action::UnknownChoice,
            };
            if checkpoint {
                let before = snapshot_session(&restored);
                let bytes = encode_session_messagepack(&restored).map_err(failure)?;
                restored = decode_session_messagepack(&asset, &bytes).map_err(failure)?;
                prop_assert_eq!(snapshot_session(&restored), before);
            }
            let stage = if let Phase::Prompt(stage) = model.phase { stage } else { 0 };
            let before = snapshot_session(&live);
            let expected = model.apply(action, &gates);
            let actual = apply(&asset, &mut live, action, stage, &gates)?;
            let resumed = apply(&asset, &mut restored, action, stage, &gates)?;
            prop_assert_eq!(observe(&actual)?, expected);
            prop_assert_eq!(&actual, &resumed);
            let after = snapshot_session(&live);
            prop_assert_eq!(&after, &snapshot_session(&restored));
            prop_assert_eq!(&after.selected_choice_history, &model.history);
            prop_assert_eq!(after.ended, matches!(model.phase, Phase::Ended));
            if actual.is_err() {
                prop_assert_eq!(after, before, "a rejected action must leave the entire session unchanged");
            }
        }
    }
}
