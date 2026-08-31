#![cfg(test)]

use serde_json::Value;
use tempfile::TempDir;

mod support;
use support::*;

#[test]
fn run_and_trace_project_the_preview_event_sequence() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> hello@aaaaaaaaaaaaaaaaaaaa\n",
            "  Hello from preview.\n",
            "-> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let fixture = write_file(temp.path(), "runtime-fixture.toml", "");

    let run_output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    run_output
        .assert_success()
        .assert_stdout_contains("line aaaaaaaaaaaaaaaaaaaa: Hello from preview.");

    let trace_output = run(recite()
        .arg("trace")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    trace_output.assert_success().assert_stderr("");
    let trace: Value = serde_json::from_slice(&trace_output.stdout).expect("trace JSON");
    let event_types = trace["events"]
        .as_array()
        .expect("trace events")
        .iter()
        .map(|event| event["type"].as_str().expect("event type"))
        .collect::<Vec<_>>();
    assert_eq!(event_types, ["line", "end"]);
    assert_eq!(trace["events"][0]["line"]["text"], "Hello from preview.");
}

#[test]
fn block_fixture_choice_is_refused_when_the_block_has_multiple_prompts() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> first@bbbbbbbbbbbbbbbbbbbb\n",
            "  First?\n",
            "  ? yes@cccccccccccccccccccc\n",
            "    Yes.\n",
            "    -> END\n",
            "> second@dddddddddddddddddddd\n",
            "  Second?\n",
            "  ? no@eeeeeeeeeeeeeeeeeeee\n",
            "    No.\n",
            "    -> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let fixture = write_file(
        temp.path(),
        "runtime-fixture.toml",
        "[choices]\nstart = 1\n",
    );
    let output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    output.assert_failure().assert_stderr_contains("ambiguous");
}

#[test]
fn preview_projection_keeps_choice_before_followup_condition() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> intro@11111111111111111111\n",
            "  Choose.\n",
            "  ? yes@22222222222222222222\n",
            "    Yes.\n",
            "    -> chosen\n",
            ":: chosen\n",
            ":if trusts(player)\n",
            "  > result@33333333333333333333\n",
            "    Result.\n",
            "-> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let fixture = write_file(
        temp.path(),
        "runtime-fixture.toml",
        "[conditions]\n\"trusts(player)\" = true\n\n[choices]\n11111111111111111111 = \"22222222222222222222\"\n",
    );
    let output = run(recite()
        .arg("trace")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    output.assert_success().assert_stderr("");
    let trace: Value = serde_json::from_slice(&output.stdout).expect("trace JSON");
    let event_types = trace["events"]
        .as_array()
        .expect("trace events")
        .iter()
        .map(|event| event["type"].as_str().expect("event type"))
        .collect::<Vec<_>>();
    assert_eq!(
        event_types,
        ["prompt", "choice_selected", "condition", "line", "end"]
    );
}

#[test]
fn multi_prompt_trace_exposes_only_line_fixture_keys_and_missing_selection_is_distinct() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> first@44444444444444444444\n",
            "  First?\n",
            "  ? yes@55555555555555555555\n",
            "    Yes.\n",
            "    -> END\n",
            "> second@66666666666666666666\n",
            "  Second?\n",
            "  ? no@77777777777777777777\n",
            "    No.\n",
            "    -> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let fixture = write_file(
        temp.path(),
        "runtime-fixture.toml",
        "[choices]\n44444444444444444444 = \"55555555555555555555\"\n66666666666666666666 = \"77777777777777777777\"\n",
    );
    let output = run(recite()
        .arg("trace")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    output.assert_success().assert_stderr("");
    let trace: Value = serde_json::from_slice(&output.stdout).expect("trace JSON");
    let prompt = trace["events"]
        .as_array()
        .expect("trace events")
        .iter()
        .find(|event| event["type"] == "prompt")
        .expect("prompt event");
    assert_eq!(
        prompt["prompt"]["identity"]["fixture_keys"],
        serde_json::json!(["44444444444444444444"])
    );

    let missing_fixture = write_file(temp.path(), "missing.toml", "");
    let output = run(recite()
        .arg("trace")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&missing_fixture));
    output
        .assert_failure()
        .assert_stderr_contains("missing a [choices]");
}
