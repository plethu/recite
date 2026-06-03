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
        "  ? help requires=(trusts(player))\n",
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
        "  ? help requires=(trusts(player))\n",
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
        "  ? help requires=(trusts(player))\n",
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
        "  ? help requires=(trusts(player))\n",
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
