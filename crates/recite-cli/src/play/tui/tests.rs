use std::path::Path;

use crate::i18n::{Messages, UiLocale};
use crate::play::preview::run_preview;
use crate::tui::{Keymap, PromptMode, TextBuffer, TuiInteractionState};
use ratatui::{Terminal, backend::TestBackend};
use recite_compiler::{CompileInput, compile_inputs};
use recite_core::CompiledDialogue;
use recite_runtime::ConditionExpectedType;

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
fn tui_shared_preview_loop_projects_real_events_and_unavailable_focus() {
    let asset = asset(concat!(
        ":: start default\n",
        "> intro@18c570b9af4d973ba876\n",
        "  Choose.\n",
        "  ? locked@c491f4cbe1944ebc5bc6 requires=(trusts(player))\n",
        "    Locked.\n",
        "    -> locked\n",
        "  ? go@c491f4cbe1944ebc5bc5\n",
        "    Go.\n",
        "    -> branch\n",
        ":: branch\n",
        ":if trusts(player)\n",
        "  ! deferred save(slot)\n",
        "  > done@d491f4cbe1944ebc5bc5\n",
        "    Done.\n",
        ":else\n",
        "  ! deferred denied(slot)\n",
        "  > denied@d491f4cbe1944ebc5bc6\n",
        "    Denied.\n",
        "-> END\n",
        ":: locked\n",
        "> locked_line@d491f4cbe1944ebc5bc7\n",
        "  Locked.\n",
        "-> END\n",
    ));
    let messages = Messages::load(&UiLocale::default()).expect("messages");
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let intents = [
        crate::tui::TuiIntent::MovePrevious,
        crate::tui::TuiIntent::Submit,
        crate::tui::TuiIntent::Submit,
        crate::tui::TuiIntent::Submit,
        crate::tui::TuiIntent::Submit,
    ];
    let mut ui = TuiPlayUi::new(
        &mut terminal,
        crate::tui::TuiSettings::default(),
        messages,
        intents,
    );
    run_preview(&asset, "start", None, &mut ui).expect("shared preview loop");

    assert!(ui.state.transcript.iter().any(|entry| {
        entry.kind == TuiTranscriptKind::Choice
            && entry.id.as_deref() == Some("c491f4cbe1944ebc5bc5")
    }));
    assert_eq!(ui.condition_answers.get("trusts(player)"), Some(&false));
    assert!(
        ui.state
            .transcript
            .iter()
            .any(|entry| entry.kind == TuiTranscriptKind::Line && entry.text == "Denied.")
    );
    let kinds = ui
        .state
        .transcript
        .iter()
        .map(|entry| entry.kind)
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        [
            TuiTranscriptKind::Condition,
            TuiTranscriptKind::Prompt,
            TuiTranscriptKind::Choice,
            TuiTranscriptKind::Condition,
            TuiTranscriptKind::Line,
            TuiTranscriptKind::End,
            TuiTranscriptKind::Deferred,
            TuiTranscriptKind::Deferred,
        ]
    );
    assert_eq!(ui.state.deferred_queue.len(), 1);
}
