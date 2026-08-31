use std::path::Path;

use crate::i18n::{Messages, UiLocale};
use crate::play::preview::PreviewPlayUi;
use crate::tui::{Keymap, PromptMode, TextBuffer, TuiInteractionState};
use ratatui::{Terminal, backend::TestBackend};
use recite_compiler::{CompileInput, compile_inputs};
use recite_core::CompiledDialogue;
use recite_runtime::ConditionExpectedType;
use recite_runtime::{
    ConditionAnswer, ConditionValue, DialogueEffectMode, PreviewCommand, PreviewEvent,
    PreviewInputs, PreviewOptions, PreviewSession,
};

use super::TuiPlayUi;
use super::interaction::enum_condition_variant;
use super::preview::condition_prompt;
use super::state::{TuiPrompt, TuiTranscriptKind};

#[test]
fn condition_prompt_uses_expected_type_specific_state() {
    let boolean = condition_prompt(
        ConditionExpectedType::Bool,
        "trusts(mira)".to_owned(),
        true,
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
        true,
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
fn tui_presenter_projects_real_preview_events_in_legacy_order() {
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
    let messages = Messages::load(&UiLocale::default()).expect("messages");
    let mut session =
        PreviewSession::new(&asset, Some("start"), PreviewOptions::new()).expect("preview session");
    let request = match session.step(PreviewInputs::default()).events() {
        [PreviewEvent::ConditionRequested(request)] => request.clone(),
        events => panic!("expected initial condition request, got {events:?}"),
    };
    let prompt = session
        .answer(
            request.id(),
            ConditionAnswer::Value(ConditionValue::Bool(true)),
            PreviewInputs::default(),
        )
        .events()
        .iter()
        .find_map(|event| match event {
            PreviewEvent::Prompt(prompt) => Some(prompt.clone()),
            _ => None,
        })
        .expect("choice prompt");
    let choice_id = prompt.choices()[0].id.clone();
    let branch_request = match session
        .dispatch(
            PreviewCommand::Choose {
                choice_id: choice_id.clone(),
            },
            PreviewInputs::default(),
        )
        .events()
    {
        [PreviewEvent::ConditionRequested(request)] => request.clone(),
        events => panic!("expected branch condition request, got {events:?}"),
    };
    let final_events = session
        .answer(
            branch_request.id(),
            ConditionAnswer::Value(ConditionValue::Bool(true)),
            PreviewInputs::default(),
        )
        .events()
        .to_vec();
    assert_eq!(final_events.len(), 4);
    assert!(matches!(
        &final_events[0],
        PreviewEvent::ConditionResult { .. }
    ));
    let PreviewEvent::ChoiceSelected {
        choice_id: selected_id,
        ..
    } = &final_events[1]
    else {
        panic!("expected selected choice")
    };
    assert_eq!(selected_id.as_str(), "c491f4cbe1944ebc5bc5");
    let PreviewEvent::DeferredEffectScheduled(effect) = &final_events[2] else {
        panic!("expected deferred effect")
    };
    assert_eq!(effect.function, "save");
    let PreviewEvent::EffectRequested(effect) = &final_events[3] else {
        panic!("expected immediate effect")
    };
    assert_eq!(effect.mode, DialogueEffectMode::Immediate);
    assert_eq!(effect.function, "ping");

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let mut ui = TuiPlayUi::new(&mut terminal, crate::tui::TuiSettings::default(), messages);
    ui.start(&asset, "start").expect("start presentation");
    ui.prepare_choice_prompt(&prompt);
    ui.selected_choice(&choice_id).expect("choice presentation");
    ui.condition_answers
        .insert("trusts(player)".to_owned(), false);
    ui.prepare_condition_prompt(&branch_request, "trusts(player)".to_owned());
    assert!(matches!(
        ui.state.prompt,
        TuiPrompt::Condition {
            selected: false,
            ..
        }
    ));
    let PreviewEvent::ConditionResult { request, result } = &final_events[0] else {
        panic!("expected condition result")
    };
    ui.condition_result(request, result)
        .expect("condition presentation");
    for event in &final_events[2..] {
        match event {
            PreviewEvent::DeferredEffectScheduled(effect) => {
                ui.deferred_effect_scheduled(effect)
                    .expect("deferred presentation");
            }
            PreviewEvent::EffectRequested(effect) => {
                ui.effect(effect).expect("effect presentation")
            }
            PreviewEvent::Line(line) => ui.line(line).expect("line presentation"),
            PreviewEvent::End { .. } => break,
            event => panic!("unexpected branch event {event:?}"),
        }
    }
    let kinds = ui
        .state
        .transcript
        .iter()
        .map(|entry| entry.kind)
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        [
            TuiTranscriptKind::Prompt,
            TuiTranscriptKind::Choice,
            TuiTranscriptKind::Condition,
            TuiTranscriptKind::Effect,
        ]
    );
    assert_eq!(ui.state.deferred_queue.len(), 1);
}
