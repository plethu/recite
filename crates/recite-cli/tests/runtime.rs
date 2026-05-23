use tempfile::TempDir;

mod support;
use support::*;

#[test]
fn run_and_trace_execute_fixture_choices_conditions_and_effect_acknowledgement() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default speaker=hazel\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? help if trusts(player)\n",
            "    Help.\n",
            "    -> help\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> leave\n",
            ":: help\n",
            "! blocking grant_item(map)\n",
            ":if has_bonus(player)\n",
            "  > bonus\n",
            "    Bonus.\n",
            ":else\n",
            "  > helped\n",
            "    Helped.\n",
            "! deferred finish(help)\n",
            "-> END\n",
            ":: leave\n",
            "> left\n",
            "  Left.\n",
            "-> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        r#"[conditions]
"trusts(player)" = true
"has_bonus(player)" = false

[choices]
intro = "help"

[effects]
auto_ack_blocking = true
"#,
    );

    let run_output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    run_output.assert_success().assert_stderr("");
    run_output.assert_stdout_contains("prompt intro: Welcome.");
    run_output.assert_stdout_contains("condition trusts(player) = true");
    run_output.assert_stdout_contains("selected choice help");
    run_output.assert_stdout_contains("effect blocking grant_item (map)");
    run_output.assert_stdout_contains("acknowledged effect");
    run_output.assert_stdout_contains("condition has_bonus(player) = false");
    run_output.assert_stdout_contains("line helped: Helped.");
    run_output.assert_stdout_contains("deferred effects:");
    run_output.assert_stdout_contains("finish (help)");

    let index_fixture = write_file(
        temp.path(),
        "fixture-index.toml",
        r#"[conditions]
"trusts(player)" = true
"has_bonus(player)" = false

[choices]
start = 1

[effects]
auto_ack_blocking = true
"#,
    );
    let index_output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&index_fixture));
    index_output.assert_success().assert_stderr("");
    index_output.assert_stdout_contains("selected choice help");
    index_output.assert_stdout_contains("line helped: Helped.");

    let trace_output = run(recite()
        .arg("trace")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    trace_output.assert_success().assert_stderr("");
    let second_trace_output = run(recite()
        .arg("trace")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    second_trace_output.assert_success().assert_stderr("");
    assert_eq!(
        trace_output.stdout, second_trace_output.stdout,
        "trace output must be byte-stable for identical asset and fixture inputs"
    );
    let trace: serde_json::Value =
        serde_json::from_slice(&trace_output.stdout).expect("trace is JSON");
    assert_eq!(trace["block"], "start");
    assert_eq!(trace["final_deferred_effects"][0]["function"], "finish");

    let events = trace["events"].as_array().expect("events array");
    assert!(events.iter().any(|event| {
        event["type"] == "condition"
            && event["condition"]["query"] == "trusts(player)"
            && event["condition"]["result"] == true
    }));
    assert!(events.iter().any(|event| {
        event["type"] == "condition"
            && event["condition"]["query"] == "has_bonus(player)"
            && event["condition"]["result"] == false
    }));
    assert!(events.iter().any(|event| {
        event["type"] == "line"
            && event["line"]["id"] == "helped"
            && event["line"]["text"] == "Helped."
    }));
    assert!(events.iter().any(|event| {
        event["type"] == "prompt"
            && event["prompt"]["identity"]["fixture_keys"]
                .as_array()
                .expect("fixture keys")
                .iter()
                .any(|key| key == "intro")
            && event["prompt"]["choices"]
                .as_array()
                .expect("prompt choices")
                .iter()
                .any(|choice| choice["id"] == "help" && choice["is_available"] == true)
    }));
    assert!(events.iter().any(|event| {
        event["type"] == "choice_selected"
            && event["prompt"]["line"] == "intro"
            && event["choice"] == "help"
    }));
    assert!(events.iter().any(|event| {
        event["type"] == "effect"
            && event["effect"]["mode"] == "blocking"
            && event["effect"]["function"] == "grant_item"
    }));
    assert!(
        events
            .iter()
            .any(|event| { event["type"] == "acknowledgement" && event["result"] == "completed" })
    );
}

#[test]
fn run_reports_fixture_and_blocking_acknowledgement_failures() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default speaker=hazel\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? help\n",
            "    Help.\n",
            "    -> help\n",
            ":: help\n",
            "! blocking grant_item(map)\n",
            "-> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let no_ack_fixture = write_file(
        temp.path(),
        "no-ack.toml",
        r#"[choices]
intro = "help"
"#,
    );

    let output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&no_ack_fixture));
    output.assert_failure();
    output.assert_stderr_contains("auto_ack_blocking = true");

    let unknown_field_fixture = write_file(
        temp.path(),
        "unknown-field.toml",
        r#"[choices]
intro = "help"

[effects]
auto_ack_blocking = true
surprise = true
"#,
    );
    let output = run(recite()
        .arg("trace")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&unknown_field_fixture));
    output.assert_failure();
    output.assert_stderr_contains("unknown field");
}
