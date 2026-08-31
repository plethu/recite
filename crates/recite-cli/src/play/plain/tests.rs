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
    run_preview(asset, "start", None, &mut ui)?;
    Ok(String::from_utf8(output).expect("utf8"))
}

#[test]
fn plain_play_selects_choice_by_index_answers_condition_and_acknowledges_blocking_effect() {
    let asset = asset(concat!(
        ":: start default\n",
        "> intro@2f9e740ea0cce1b9441c\n",
        "  Welcome.\n",
        "  ? help@3be9e1e61bb8abb63ec6 requires=(trusts(player))\n",
        "    Help.\n",
        "    -> help\n",
        "  ? leave@6f7894fabc68b2d600c7\n",
        "    Leave.\n",
        "    -> END\n",
        ":: help\n",
        "! blocking grant_item(map)\n",
        "> helped@9c6ba1d11f766dc13797\n",
        "  Helped.\n",
        "! deferred finish(help)\n",
        "-> END\n",
    ));

    let output = run_plain(&asset, "y\n1\nack\n").expect("play succeeds");

    assert!(output.contains("condition trusts(player) = true"));
    assert!(output.contains("selected choice 3be9e1e61bb8abb63ec6"));
    assert!(output.contains("effect blocking"));
    assert!(output.contains("acknowledged effect"));
    assert!(output.contains("line 9c6ba1d11f766dc13797: Helped."));
    assert!(output.contains("deferred effects:"));
}

#[test]
fn plain_play_selects_choice_by_id() {
    let asset = asset(concat!(
        ":: start default\n",
        "> intro@bec122cc9b92a43d2f0c\n",
        "  Welcome.\n",
        "  ? help@cb1ac86a99e43bc3a86a\n",
        "    Help.\n",
        "    -> help\n",
        "  ? leave@9e78bd9ca3387d1476ee\n",
        "    Leave.\n",
        "    -> END\n",
        ":: help\n",
        "> helped@8b72a27c0c90e28bdfcc\n",
        "  Helped.\n",
        "-> END\n",
    ));

    let output = run_plain(&asset, "cb1ac86a99e43bc3a86a\n").expect("play succeeds");

    assert!(output.contains("selected choice cb1ac86a99e43bc3a86a"));
    assert!(output.contains("line 8b72a27c0c90e28bdfcc: Helped."));
}

#[test]
fn plain_play_can_select_numeric_choice_id() {
    let asset = asset(concat!(
        ":: start default\n",
        "> intro@9e5b3c1f047f98db3b09\n",
        "  Welcome.\n",
        "  ? skip@0d60b6fe779af793161a\n",
        "    Skip.\n",
        "    -> skip\n",
        "  ? numeric_choice@20000000000000000000\n",
        "    Numeric.\n",
        "    -> numeric\n",
        ":: skip\n",
        "> skipped@35f33407a9b21e947b0e\n",
        "  Skipped.\n",
        "-> END\n",
        ":: numeric\n",
        "> numeric_line@34636e6a8d270c337e37\n",
        "  Numeric ID selected.\n",
        "-> END\n",
    ));

    let output = run_plain(&asset, "2\n").expect("play succeeds");

    assert!(output.contains("selected choice 20000000000000000000"));
    assert!(output.contains("line 34636e6a8d270c337e37: Numeric ID selected."));
    assert!(!output.contains("selected choice 0d60b6fe779af793161a"));
}

#[test]
fn plain_play_reprompts_after_invalid_choice_and_condition_input() {
    let asset = asset(concat!(
        ":: start default\n",
        "> intro@ae7d258bef8210e0095a\n",
        "  Welcome.\n",
        "  ? help@50a096ad4aa3edc4b725 requires=(trusts(player))\n",
        "    Help.\n",
        "    -> END\n",
    ));

    let output = run_plain(&asset, "maybe\ny\n\nbad id\n99\n1\n").expect("play succeeds");

    assert!(output.contains("invalid input: enter y or n"));
    assert!(output.contains("invalid input: choice selection cannot be empty"));
    assert!(output.contains("invalid input: choice ID `bad id` is not available here"));
    assert!(output.contains("invalid input: choice index 99 is out of range"));
    assert!(output.contains("selected choice 50a096ad4aa3edc4b725"));
}

#[test]
fn plain_play_reprompts_for_unavailable_choice_without_recording_selection() {
    let asset = asset(concat!(
        ":: start default\n",
        "> intro@48eb683c9e5af8cdb826\n",
        "  Welcome.\n",
        "  ? help@ce6c3837e15ee211b4b8 requires=(trusts(player))\n",
        "    Help.\n",
        "    -> help\n",
        "  ? leave@b02f1c80cdf6d2f8cacd\n",
        "    Leave.\n",
        "    -> leave\n",
        ":: help\n",
        "> helped@dd0cde0b5848f0246934\n",
        "  Helped.\n",
        "-> END\n",
        ":: leave\n",
        "> left@67fa7d6dcb35553cc928\n",
        "  Left.\n",
        "-> END\n",
    ));

    let output = run_plain(&asset, "n\n1\nb02f1c80cdf6d2f8cacd\n").expect("play succeeds");

    assert!(output.contains("condition trusts(player) = false"));
    assert!(output.contains("invalid input: choice `ce6c3837e15ee211b4b8` is unavailable"));
    assert!(!output.contains("selected choice ce6c3837e15ee211b4b8"));
    assert!(output.contains("selected choice b02f1c80cdf6d2f8cacd"));
    assert!(output.contains("line 67fa7d6dcb35553cc928: Left."));
}

#[test]
fn plain_play_reports_eof() {
    let asset = asset(concat!(
        ":: start default\n",
        "> intro@6c422418c44d026d2667\n",
        "  Welcome.\n",
        "  ? help@60bac9d055704058ba38\n",
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
        "> intro@42551710726ce2a25b3d\n",
        "  Welcome.\n",
        "  ? help@439bac3d440b7dad1e6c requires=(trusts(player))\n",
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
        "> intro@d4015ea1a2b18d0979f4\n",
        "  Welcome.\n",
        "  ? help@8d41a6f3bba2b4c2e660\n",
        "    Help.\n",
        "    -> help\n",
        ":: help\n",
        ":if trusts(player)\n",
        "  > helped@33772e0e0fbf80051af0\n",
        "    Helped.\n",
        "-> END\n",
    ));

    let error = run_plain(&asset, "8d41a6f3bba2b4c2e660\n").expect_err("eof fails");

    assert!(matches!(
        error,
        CliError::PlayEof {
            field: "condition answer"
        }
    ));
}

#[test]
fn plain_preview_presents_choice_before_followup_condition() {
    let asset = asset(concat!(
        ":: start default\n",
        "> intro@d4015ea1a2b18d0979f4\n",
        "  Welcome.\n",
        "  ? help@8d41a6f3bba2b4c2e660\n",
        "    Help.\n",
        "    -> help\n",
        ":: help\n",
        ":if trusts(player)\n",
        "  > helped@33772e0e0fbf80051af0\n",
        "    Helped.\n",
        "-> END\n",
    ));

    let output = run_plain(&asset, "1\ny\n").expect("play succeeds");
    let selected = output
        .find("selected choice 8d41a6f3bba2b4c2e660")
        .expect("selected choice");
    let condition = output
        .rfind("condition trusts(player) = true")
        .expect("follow-up condition result");
    assert!(selected < condition);
}

#[test]
fn plain_preview_preserves_typed_event_order_for_condition_and_choice() {
    let asset = asset(concat!(
        ":: start default\n",
        "> intro@11111111111111111111\n",
        "  Welcome.\n",
        "  ? continue@22222222222222222222 requires=(trusts(player))\n",
        "    Continue.\n",
        "    -> END\n",
    ));
    let output = run_plain(&asset, "y\n1\n").expect("play succeeds");
    let condition = output
        .find("condition trusts(player) = true")
        .expect("condition");
    let prompt = output
        .find("prompt 11111111111111111111: Welcome.")
        .expect("prompt");
    let selected = output
        .find("selected choice 22222222222222222222")
        .expect("selection");
    let end = output.find("end").expect("end");
    assert!(condition < prompt && prompt < selected && selected < end);
}
