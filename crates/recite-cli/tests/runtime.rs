#![cfg(test)]

use tempfile::TempDir;

mod support;
use support::*;
#[path = "runtime/interpolation.rs"]
mod interpolation;

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
            "> @h3nt0d7\n",
            "  Welcome.\n",
            "  ? requires=(trusts(player)) @rkj778k\n",
            "    Help.\n",
            "    -> help\n",
            "  ? @wdpx2ys\n",
            "    Leave.\n",
            "    -> leave\n",
            ":: help\n",
            "! blocking grant_item(map)\n",
            ":if has_bonus(player)\n",
            "  > @bonus\n",
            "    Bonus.\n",
            ":else\n",
            "  > @ntz2ny0\n",
            "    Helped.\n",
            "! deferred finish(help)\n",
            "-> END\n",
            ":: leave\n",
            "> @left\n",
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
h3nt0d7 = "rkj778k"

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
    run_output.assert_stdout_contains("prompt h3nt0d7: Welcome.");
    run_output.assert_stdout_contains("condition trusts(player) = true");
    run_output.assert_stdout_contains("selected choice rkj778k");
    run_output.assert_stdout_contains("effect blocking grant_item (map)");
    run_output.assert_stdout_contains("acknowledged effect");
    run_output.assert_stdout_contains("condition has_bonus(player) = false");
    run_output.assert_stdout_contains("line ntz2ny0: Helped.");
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
    index_output.assert_stdout_contains("selected choice rkj778k");
    index_output.assert_stdout_contains("line ntz2ny0: Helped.");

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
            && event["line"]["id"] == "ntz2ny0"
            && event["line"]["text"] == "Helped."
    }));
    assert!(events.iter().any(|event| {
        event["type"] == "prompt"
            && event["prompt"]["identity"]["fixture_keys"]
                .as_array()
                .expect("fixture keys")
                .iter()
                .any(|key| key == "h3nt0d7")
            && event["prompt"]["choices"]
                .as_array()
                .expect("prompt choices")
                .iter()
                .any(|choice| choice["id"] == "rkj778k" && choice["is_available"] == true)
    }));
    assert!(events.iter().any(|event| {
        event["type"] == "choice_selected"
            && event["prompt"]["line"] == "h3nt0d7"
            && event["choice"] == "rkj778k"
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
            "> @h7btn6w\n",
            "  Welcome.\n",
            "  ? @kg216b7\n",
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
h7btn6w = "kg216b7"
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
h7btn6w = "kg216b7"

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
fn trace_preserves_typed_literal_availability_reason_args() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> @h0hekng\n",
            "  Welcome.\n",
            "  ? requires=(can_answer()) @answer\n",
            "    Answer.\n",
            "    -> END\n",
            "  ? @vs56zr5\n",
            "    Leave.\n",
            "    -> END\n",
        ),
    );
    let schema = write_file(
        temp.path(),
        "schema.json",
        r#"{
  "schema_version": 1,
  "types": {
    "mood": { "kind": "enum", "values": ["sad"] }
  },
  "registries": {
    "actor": { "values": ["hazel"] }
  },
  "speakers": {
    "rhea": {}
  },
  "conditions": {
    "can_answer": {
      "availability_reason": {
        "reason": "answer_blocked",
        "args": {
          "actor": "hazel",
          "speaker": "rhea",
          "mood": "sad",
          "count": 3,
          "weight": 1.5,
          "enabled": true
        }
      }
    }
  },
  "availability_reasons": {
    "answer_blocked": {
      "template": "{actor} {speaker} {mood} {count} {weight} {enabled}",
      "params": [
        { "name": "actor", "type": "registry:actor" },
        { "name": "speaker", "type": "speaker" },
        { "name": "mood", "type": "enum:mood" },
        { "name": "count", "type": "int" },
        { "name": "weight", "type": "float" },
        { "name": "enabled", "type": "bool" }
      ]
    }
  }
}"#,
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", Some(&schema));
    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        r#"[conditions]
"can_answer()" = false

[choices]
h0hekng = "vs56zr5"
"#,
    );

    let trace_output = run(recite()
        .arg("trace")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    trace_output.assert_success().assert_stderr("");
    let trace: serde_json::Value =
        serde_json::from_slice(&trace_output.stdout).expect("trace is JSON");
    let choices = trace["events"][1]["prompt"]["choices"]
        .as_array()
        .expect("prompt choices");
    let answer = choices
        .iter()
        .find(|choice| choice["id"] == "answer")
        .expect("answer choice");

    assert_eq!(
        answer["availability"]["reason_tree"]["value"]["args"],
        serde_json::json!([
            { "name": "actor", "value": { "type": "string", "value": "hazel" } },
            { "name": "count", "value": { "type": "integer", "value": 3 } },
            { "name": "enabled", "value": { "type": "boolean", "value": true } },
            { "name": "mood", "value": { "type": "string", "value": "sad" } },
            { "name": "speaker", "value": { "type": "string", "value": "rhea" } },
            { "name": "weight", "value": { "type": "float", "value": 1.5 } }
        ])
    );
    assert_eq!(
        answer["availability"]["reason_tree"]["value"]["text"],
        "hazel rhea sad 3 1.5 true"
    );

    let run_output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    run_output.assert_success();
    assert_stdout_sequence(&run_output, ["  [1] answer: Answer. (unavailable)\n"]);
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
            "    > @tired_line\n",
            "      Tired.\n",
            "      ? @rest\n",
            "        Rest.\n",
            "        -> END\n",
            "  :case _\n",
            "    > @fallback_line\n",
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
            "> @spaamtv\n",
            "  Welcome.\n",
            "  ? requires=(trusts(player)) @c6c3pxx\n",
            "    Help.\n",
            "    -> help\n",
            "  ? @xag6ece\n",
            "    Leave.\n",
            "    -> END\n",
            ":: help\n",
            "! blocking grant_item(map)\n",
            "> @aqp0m89\n",
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
        .write_all(b"y\nc6c3pxx\n\n")
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");

    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("play asset=");
    output.assert_stdout_contains("condition trusts(player) = true");
    output.assert_stdout_contains("selected choice c6c3pxx");
    output.assert_stdout_contains("effect blocking id=");
    output.assert_stdout_contains("acknowledged effect");
    output.assert_stdout_contains("line aqp0m89: Helped.");
    output.assert_stdout_contains("deferred effects:");
    assert_stdout_sequence(
        &output,
        [
            "play asset=",
            "condition trusts(player) = true",
            "prompt spaamtv: Welcome.",
            "selected choice c6c3pxx",
            "effect blocking id=",
            "acknowledged effect",
            "line aqp0m89: Helped.",
            "end",
            "deferred effects:",
        ],
    );

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
        .write_all(b"n\nxag6ece\n")
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");

    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("condition trusts(player) = false");
    output.assert_stdout_contains("selected choice xag6ece");
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

fn assert_stdout_sequence<const N: usize>(output: &std::process::Output, snippets: [&str; N]) {
    let stdout = stdout(output);
    let mut cursor = 0;
    for snippet in snippets {
        let Some(offset) = stdout[cursor..].find(snippet) else {
            panic!("stdout did not contain {snippet:?} after byte {cursor}\nstdout:\n{stdout}");
        };
        cursor += offset + snippet.len();
    }
}

#[test]
fn play_ui_locale_config_falls_back_to_default_catalog_and_rejects_bad_locale() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> @dqqhcve\n",
            "  Welcome.\n",
            "  ? @g2mnzyz\n",
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
        .write_all(b"g2mnzyz\n")
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");

    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("prompt dqqhcve: Welcome.");
    output.assert_stdout_contains("selected choice g2mnzyz");

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
dqqhcve = "g2mnzyz"
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
    output.assert_stdout_contains("selected choice g2mnzyz");
}
