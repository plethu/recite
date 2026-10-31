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
            "> @m3ffvrt\n",
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
        .assert_stdout_contains("line m3ffvrt: Hello from preview.");

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
            "> @qdjf739\n",
            "  First?\n",
            "  ? @z1pshhd\n",
            "    Yes.\n",
            "    -> END\n",
            "> @yt5egpn\n",
            "  Second?\n",
            "  ? @no\n",
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
            "> @ffee6p8\n",
            "  Choose.\n",
            "  ? @rthh7ab\n",
            "    Yes.\n",
            "    -> chosen\n",
            ":: chosen\n",
            ":if trusts(player)\n",
            "  > @bqyvpqq\n",
            "    Result.\n",
            "-> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let fixture = write_file(
        temp.path(),
        "runtime-fixture.toml",
        "[conditions]\n\"trusts(player)\" = true\n\n[choices]\nffee6p8 = \"rthh7ab\"\n",
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
            "> @z0g6c80\n",
            "  First?\n",
            "  ? @yes\n",
            "    Yes.\n",
            "    -> END\n",
            "> @h2zyg2g\n",
            "  Second?\n",
            "  ? @wx4me54\n",
            "    No.\n",
            "    -> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let fixture = write_file(
        temp.path(),
        "runtime-fixture.toml",
        "[choices]\nz0g6c80 = \"yes\"\nh2zyg2g = \"wx4me54\"\n",
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
        serde_json::json!(["z0g6c80"])
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
