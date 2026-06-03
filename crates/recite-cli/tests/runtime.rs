#![cfg(test)]

use tempfile::TempDir;

mod support;
use support::*;

use std::io::Write;
use std::process::Stdio;

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
            "  ? help requires=(trusts(player))\n",
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

#[test]
fn run_trace_and_play_plain_execute_match_conditions() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            ":match thread_stage(thread)\n",
            "  :case tired\n",
            "    > tired_line\n",
            "      Tired.\n",
            "      ? rest\n",
            "        Rest.\n",
            "        -> END\n",
            "  :case _\n",
            "    > fallback_line\n",
            "      Fallback.\n",
            "-> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        r#"[conditions]
"thread_stage(thread)" = { enum = "tired" }

[choices]
tired_line = "rest"
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
    run_output.assert_stdout_contains("condition thread_stage(thread) = enum tired");
    run_output.assert_stdout_contains("prompt tired_line: Tired.");
    run_output.assert_stdout_contains("selected choice rest");

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
    assert_eq!(trace_output.stdout, second_trace_output.stdout);
    let trace: serde_json::Value =
        serde_json::from_slice(&trace_output.stdout).expect("trace is JSON");
    let events = trace["events"].as_array().expect("events array");
    assert!(events.iter().any(|event| {
        event["type"] == "condition"
            && event["condition"]["query"] == "thread_stage(thread)"
            && event["condition"]["result"]["enum"] == "tired"
    }));
    assert!(events.iter().any(|event| {
        event["type"] == "prompt"
            && event["prompt"]["identity"]["line"] == "tired_line"
            && event["prompt"]["identity"]["fixture_keys"][0] == "tired_line"
    }));

    let missing_fixture = write_file(temp.path(), "missing.toml", "");
    let output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&missing_fixture));
    output.assert_failure();
    output.assert_stderr_contains("fixture is missing condition `thread_stage(thread)`");
    output.assert_stderr_not_contains("does not support");

    let mut child = recite()
        .arg("play")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--ui")
        .arg("plain")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn recite play");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"tired\nrest\n")
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");

    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("condition thread_stage(thread) = tired");
    output.assert_stdout_contains("prompt tired_line: Tired.");
    output.assert_stdout_contains("selected choice rest");
}

#[test]
fn play_plain_accepts_piped_input_and_keeps_run_trace_stable() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
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
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);

    let mut child = recite()
        .arg("play")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--ui")
        .arg("plain")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn recite play");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"y\nhelp\n\n")
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");

    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("play asset=");
    output.assert_stdout_contains("condition trusts(player) = true");
    output.assert_stdout_contains("selected choice help");
    output.assert_stdout_contains("effect blocking id=");
    output.assert_stdout_contains("acknowledged effect");
    output.assert_stdout_contains("line helped: Helped.");
    output.assert_stdout_contains("deferred effects:");

    let mut child = recite()
        .arg("play")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn recite play auto");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"n\nleave\n")
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");

    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("condition trusts(player) = false");
    output.assert_stdout_contains("selected choice leave");
    output.assert_stdout_contains("end");

    let output = run(recite()
        .arg("play")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--ui")
        .arg("tui"));
    output.assert_failure();
    output.assert_stderr_contains("use --ui plain");
}

#[test]
fn play_ui_locale_config_falls_back_to_default_catalog_and_rejects_bad_locale() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);

    let fallback_config = write_file(
        temp.path(),
        "config-fallback.toml",
        r#"[ui]
locale = "en-GB"
"#,
    );
    let mut child = recite()
        .arg("play")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--ui")
        .arg("plain")
        .env("RECITE_CONFIG", &fallback_config)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn recite play");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"leave\n")
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");

    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("prompt intro: Welcome.");
    output.assert_stdout_contains("selected choice leave");

    let bad_config = write_file(
        temp.path(),
        "config-bad.toml",
        r#"[ui]
locale = "not a locale"
"#,
    );
    let output = run(recite()
        .arg("play")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--ui")
        .arg("plain")
        .env("RECITE_CONFIG", &bad_config));

    output.assert_failure();
    output.assert_stderr_contains("invalid [ui].locale");

    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        r#"[choices]
intro = "leave"
"#,
    );
    let output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture)
        .env("RECITE_CONFIG", &bad_config));

    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("selected choice leave");
}
