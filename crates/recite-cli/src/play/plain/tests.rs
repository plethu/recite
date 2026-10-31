use super::*;
use std::io::{self, Cursor, Read};
use std::path::Path;

use recite_compiler::compile::{CompileInput, compile_inputs};

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

fn choice_branch_asset() -> CompiledDialogue {
    asset(concat!(
        ":: start default\n",
        "> @d94mpnm\n",
        "  Welcome.\n",
        "  ? @h01fyzq\n",
        "    Help.\n",
        "    -> help\n",
        ":: help\n",
        ":if trusts(player)\n",
        "  > @znzk9h1\n",
        "    Helped.\n",
        "-> END\n",
    ))
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
        "> @c5msqqx\n",
        "  Welcome.\n",
        "  ? requires=(trusts(player)) @vhfgkr9\n",
        "    Help.\n",
        "    -> help\n",
        "  ? @ke33y54\n",
        "    Leave.\n",
        "    -> END\n",
        ":: help\n",
        "! blocking grant_item(map)\n",
        "> @mr71a02\n",
        "  Helped.\n",
        "! deferred finish(help)\n",
        "-> END\n",
    ));

    let output = run_plain(&asset, "y\n1\nack\n").expect("play succeeds");

    assert!(output.contains("condition trusts(player) = true"));
    assert!(output.contains("selected choice vhfgkr9"));
    assert!(output.contains("effect blocking"));
    assert!(output.contains("acknowledged effect"));
    assert!(output.contains("line mr71a02: Helped."));
    assert!(output.contains("deferred effects:"));
}

#[test]
fn plain_play_selects_choice_by_id() {
    let asset = asset(concat!(
        ":: start default\n",
        "> @bwhjpcf\n",
        "  Welcome.\n",
        "  ? @zf9v6at\n",
        "    Help.\n",
        "    -> help\n",
        "  ? @bqtcyhd\n",
        "    Leave.\n",
        "    -> END\n",
        ":: help\n",
        "> @z663db9\n",
        "  Helped.\n",
        "-> END\n",
    ));

    let output = run_plain(&asset, "zf9v6at\n").expect("play succeeds");

    assert!(output.contains("selected choice zf9v6at"));
    assert!(output.contains("line z663db9: Helped."));
}

#[test]
fn plain_play_can_select_numeric_choice_id() {
    let asset = asset(concat!(
        ":: start default\n",
        "> @ppppxf8\n",
        "  Welcome.\n",
        "  ? @skip\n",
        "    Skip.\n",
        "    -> skip\n",
        "  ? @numeric_choice\n",
        "    Numeric.\n",
        "    -> numeric\n",
        ":: skip\n",
        "> @skipped\n",
        "  Skipped.\n",
        "-> END\n",
        ":: numeric\n",
        "> @numeric_line\n",
        "  Numeric ID selected.\n",
        "-> END\n",
    ));

    let output = run_plain(&asset, "2\n").expect("play succeeds");

    assert!(output.contains("selected choice numeric_choice"));
    assert!(output.contains("line numeric_line: Numeric ID selected."));
    assert!(!output.contains("selected choice skip"));
}

#[test]
fn plain_play_reprompts_after_invalid_choice_and_condition_input() {
    let asset = asset(concat!(
        ":: start default\n",
        "> @ae17x7j\n",
        "  Welcome.\n",
        "  ? requires=(trusts(player)) @e553mvb\n",
        "    Help.\n",
        "    -> END\n",
    ));

    let output = run_plain(&asset, "maybe\ny\n\nbad id\n99\n1\n").expect("play succeeds");

    assert!(output.contains("invalid input: enter y or n"));
    assert!(output.contains("invalid input: choice selection cannot be empty"));
    assert!(output.contains("invalid input: choice ID `bad id` is not available here"));
    assert!(output.contains("invalid input: choice index 99 is out of range"));
    assert!(output.contains("selected choice e553mvb"));
}

#[test]
fn plain_play_reprompts_for_unavailable_choice_without_recording_selection() {
    let asset = asset(concat!(
        ":: start default\n",
        "> @j9bkg1b\n",
        "  Welcome.\n",
        "  ? requires=(trusts(player)) @pwmkh0r\n",
        "    Help.\n",
        "    -> help\n",
        "  ? @syp5q7k\n",
        "    Leave.\n",
        "    -> leave\n",
        ":: help\n",
        "> @mzq4fed\n",
        "  Helped.\n",
        "-> END\n",
        ":: leave\n",
        "> @left\n",
        "  Left.\n",
        "-> END\n",
    ));

    let output = run_plain(&asset, "n\n1\nsyp5q7k\n").expect("play succeeds");

    assert!(output.contains("condition trusts(player) = false"));
    assert!(output.contains("invalid input: choice `pwmkh0r` is unavailable"));
    assert!(!output.contains("selected choice pwmkh0r"));
    assert!(output.contains("selected choice syp5q7k"));
    assert!(output.contains("line left: Left."));
}

#[test]
fn plain_play_reports_eof() {
    let asset = asset(concat!(
        ":: start default\n",
        "> @vx31qts\n",
        "  Welcome.\n",
        "  ? @h3wdq60\n",
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
        "> @vshy8te\n",
        "  Welcome.\n",
        "  ? requires=(trusts(player)) @kgb11f4\n",
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
fn plain_preview_preserves_typed_event_order_for_condition_and_choice() {
    let asset = asset(concat!(
        ":: start default\n",
        "> @ffee6p8\n",
        "  Welcome.\n",
        "  ? requires=(trusts(player)) @rthh7ab\n",
        "    Continue.\n",
        "    -> END\n",
    ));
    let output = run_plain(&asset, "y\n1\n").expect("play succeeds");
    let condition = output
        .find("condition trusts(player) = true")
        .expect("condition");
    let prompt = output.find("prompt ffee6p8: Welcome.").expect("prompt");
    let selected = output.find("selected choice rthh7ab").expect("selection");
    let end = output.find("end").expect("end");
    assert!(condition < prompt && prompt < selected && selected < end);
}

fn run_plain_capture(asset: &CompiledDialogue, input: &str) -> (Result<(), CliError>, String) {
    let mut input = input.as_bytes();
    let mut output = Vec::new();
    let messages = Messages::load(&crate::i18n::UiLocale::default()).expect("messages");
    let mut ui = PlainPlayUi::new(&mut input, &mut output, &messages);
    let result = run_preview(asset, "start", None, &mut ui);
    (result, String::from_utf8(output).expect("utf8"))
}

struct ErrorAfterInput {
    input: Cursor<Vec<u8>>,
    failed: bool,
}

impl ErrorAfterInput {
    fn new(input: &str) -> Self {
        Self {
            input: Cursor::new(input.as_bytes().to_vec()),
            failed: false,
        }
    }
}

impl Read for ErrorAfterInput {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.input.read(buffer)?;
        if read == 0 && !self.failed {
            self.failed = true;
            return Err(io::Error::other("input failed"));
        }
        Ok(read)
    }
}

#[test]
fn plain_play_reports_post_choice_condition_eof_as_cli_error() {
    let asset = choice_branch_asset();

    let (result, output) = run_plain_capture(&asset, "h01fyzq\n");
    let error = result.expect_err("eof fails");

    assert!(matches!(
        error,
        CliError::PlayEof {
            field: "condition answer"
        }
    ));
    assert_eq!(output.matches("selected choice h01fyzq").count(), 1);
    assert!(
        output
            .find("selected choice h01fyzq")
            .expect("accepted choice")
            < output
                .find("condition trusts(player)")
                .expect("condition prompt")
    );
}

#[test]
fn plain_play_reports_post_choice_condition_io_without_duplicate_selection() {
    let asset = choice_branch_asset();
    let mut input = ErrorAfterInput::new("h01fyzq\n");
    let mut output = Vec::new();
    let messages = Messages::load(&crate::i18n::UiLocale::default()).expect("messages");
    let mut ui = PlainPlayUi::new(&mut input, &mut output, &messages);
    let result = run_preview(&asset, "start", None, &mut ui);
    let output = String::from_utf8(output).expect("utf8");

    assert!(matches!(result, Err(CliError::Io(_))));
    assert_eq!(output.matches("selected choice h01fyzq").count(), 1);
    assert!(
        output
            .find("selected choice h01fyzq")
            .expect("accepted choice")
            < output
                .find("condition trusts(player)")
                .expect("condition prompt")
    );
}

#[test]
fn plain_preview_presents_choice_before_followup_condition() {
    let asset = choice_branch_asset();

    let (result, output) = run_plain_capture(&asset, "1\ny\n");
    result.expect("play succeeds");
    let selected = output
        .find("selected choice h01fyzq")
        .expect("selected choice");
    let condition = output
        .rfind("condition trusts(player) = true")
        .expect("follow-up condition result");
    assert_eq!(output.matches("selected choice h01fyzq").count(), 1);
    assert!(selected < condition);
}
