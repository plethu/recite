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
            "> intro@1fae7e4dcc2f49dda604\n",
            "  Welcome.\n",
            "  ? help@b2c08cc280c726da34bf requires=(trusts(player))\n",
            "    Help.\n",
            "    -> help\n",
            "  ? leave@94f6053553c00b4d01f0\n",
            "    Leave.\n",
            "    -> leave\n",
            ":: help\n",
            "! blocking grant_item(map)\n",
            ":if has_bonus(player)\n",
            "  > bonus@88dd946ec2db5dcad0f5\n",
            "    Bonus.\n",
            ":else\n",
            "  > helped@471993b01dc658723ed5\n",
            "    Helped.\n",
            "! deferred finish(help)\n",
            "-> END\n",
            ":: leave\n",
            "> left@363b6e1d1f5b8cf263ad\n",
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
1fae7e4dcc2f49dda604 = "b2c08cc280c726da34bf"

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
    run_output.assert_stdout_contains("prompt 1fae7e4dcc2f49dda604: Welcome.");
    run_output.assert_stdout_contains("condition trusts(player) = true");
    run_output.assert_stdout_contains("selected choice b2c08cc280c726da34bf");
    run_output.assert_stdout_contains("effect blocking grant_item (map)");
    run_output.assert_stdout_contains("acknowledged effect");
    run_output.assert_stdout_contains("condition has_bonus(player) = false");
    run_output.assert_stdout_contains("line 471993b01dc658723ed5: Helped.");
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
    index_output.assert_stdout_contains("selected choice b2c08cc280c726da34bf");
    index_output.assert_stdout_contains("line 471993b01dc658723ed5: Helped.");

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
            && event["line"]["id"] == "471993b01dc658723ed5"
            && event["line"]["text"] == "Helped."
    }));
    assert!(events.iter().any(|event| {
        event["type"] == "prompt"
            && event["prompt"]["identity"]["fixture_keys"]
                .as_array()
                .expect("fixture keys")
                .iter()
                .any(|key| key == "1fae7e4dcc2f49dda604")
            && event["prompt"]["choices"]
                .as_array()
                .expect("prompt choices")
                .iter()
                .any(|choice| {
                    choice["id"] == "b2c08cc280c726da34bf" && choice["is_available"] == true
                })
    }));
    assert!(events.iter().any(|event| {
        event["type"] == "choice_selected"
            && event["prompt"]["line"] == "1fae7e4dcc2f49dda604"
            && event["choice"] == "b2c08cc280c726da34bf"
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
            "> intro@f78dbc7fa0ad21e93077\n",
            "  Welcome.\n",
            "  ? help@008b6b090df272e74e52\n",
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
f78dbc7fa0ad21e93077 = "008b6b090df272e74e52"
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
f78dbc7fa0ad21e93077 = "008b6b090df272e74e52"

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
fn trace_exposes_structured_choice_availability_reasons() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> intro@82db0b1dab0a52136d77\n",
            "  Welcome.\n",
            "  ? ask_news@e8572a78baac6863754d requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint\n",
            "    Ask for private news.\n",
            "    -> END\n",
            "  ? leave@be22df697e7ee4d7ba1b\n",
            "    Leave.\n",
            "    -> END\n",
        ),
    );
    let schema = write_file(
        temp.path(),
        "schema.json",
        include_str!("../../../fixtures/schema/valid/generated_manifest.json"),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", Some(&schema));
    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        r#"[conditions]
"trust_gte(hazel, rhea, 3)" = false

[choices]
82db0b1dab0a52136d77 = "be22df697e7ee4d7ba1b"
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
    let ask_news = choices
        .iter()
        .find(|choice| choice["id"] == "e8572a78baac6863754d")
        .expect("ask_news choice");

    assert_eq!(ask_news["is_available"], false);
    assert_eq!(
        ask_news["unavailable_reason"],
        "The innkeeper is not ready to share that."
    );
    assert_eq!(ask_news["availability"]["is_available"], false);
    assert_eq!(
        ask_news["availability"]["primary_reason"]["origin"],
        serde_json::json!({
            "type": "requirement_expression",
            "source_text": "requires=(trust_gte(hazel, rhea, 3))"
        })
    );
    assert_eq!(
        ask_news["availability"]["reason_tree"],
        serde_json::json!({
            "type": "reason",
            "value": {
                "id": "trust_too_low",
                "source_text": "{subject} does not trust {target} enough ({threshold}).",
                "text": "hazel does not trust rhea enough (3).",
                "origin": {
                    "type": "condition_call",
                    "function": "trust_gte",
                    "args": [
                        { "type": "identifier", "value": "hazel" },
                        { "type": "identifier", "value": "rhea" },
                        { "type": "integer", "value": 3 }
                    ]
                },
                "args": [
                    {
                        "name": "subject",
                        "value": { "type": "identifier", "value": "hazel" }
                    },
                    {
                        "name": "target",
                        "value": { "type": "identifier", "value": "rhea" }
                    },
                    {
                        "name": "threshold",
                        "value": { "type": "integer", "value": 3 }
                    }
                ]
            }
        })
    );
}

#[test]
fn trace_preserves_typed_literal_availability_reason_args() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> intro@48c0e5d8b1edd6d2d1da\n",
            "  Welcome.\n",
            "  ? answer@c2d3dd8f18b4cb15ac82 requires=(can_answer())\n",
            "    Answer.\n",
            "    -> END\n",
            "  ? leave@333bc0a0040231b83f37\n",
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
48c0e5d8b1edd6d2d1da = "333bc0a0040231b83f37"
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
        .find(|choice| choice["id"] == "c2d3dd8f18b4cb15ac82")
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
            "    > tired_line@4759aff334455e7312d6\n",
            "      Tired.\n",
            "      ? rest@7dff78fde4c935ba8fd1\n",
            "        Rest.\n",
            "        -> END\n",
            "  :case _\n",
            "    > fallback_line@f7e50b399ad62f291e2e\n",
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
4759aff334455e7312d6 = "7dff78fde4c935ba8fd1"
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
    run_output.assert_stdout_contains("prompt 4759aff334455e7312d6: Tired.");
    run_output.assert_stdout_contains("selected choice 7dff78fde4c935ba8fd1");

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
            && event["prompt"]["identity"]["line"] == "4759aff334455e7312d6"
            && event["prompt"]["identity"]["fixture_keys"][0] == "4759aff334455e7312d6"
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
        .write_all(b"tired\n7dff78fde4c935ba8fd1\n")
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");

    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("condition thread_stage(thread) = tired");
    output.assert_stdout_contains("prompt 4759aff334455e7312d6: Tired.");
    output.assert_stdout_contains("selected choice 7dff78fde4c935ba8fd1");
}

#[test]
fn play_plain_accepts_piped_input_and_keeps_run_trace_stable() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> intro@b9be382cc070fa4ffa18\n",
            "  Welcome.\n",
            "  ? help@bc8fdb2ff18171b53d0a requires=(trusts(player))\n",
            "    Help.\n",
            "    -> help\n",
            "  ? leave@175f47f391468d580eeb\n",
            "    Leave.\n",
            "    -> END\n",
            ":: help\n",
            "! blocking grant_item(map)\n",
            "> helped@97b6222c6841cf9b4788\n",
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
        .write_all(b"y\nbc8fdb2ff18171b53d0a\n\n")
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");

    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("play asset=");
    output.assert_stdout_contains("condition trusts(player) = true");
    output.assert_stdout_contains("selected choice bc8fdb2ff18171b53d0a");
    output.assert_stdout_contains("effect blocking id=");
    output.assert_stdout_contains("acknowledged effect");
    output.assert_stdout_contains("line 97b6222c6841cf9b4788: Helped.");
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
        .write_all(b"n\n175f47f391468d580eeb\n")
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");

    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("condition trusts(player) = false");
    output.assert_stdout_contains("selected choice 175f47f391468d580eeb");
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
            "> intro@c8cced2046cf8ac8f7b0\n",
            "  Welcome.\n",
            "  ? leave@1cf4c868fe680ee4bb4c\n",
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
        .write_all(b"1cf4c868fe680ee4bb4c\n")
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");

    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("prompt c8cced2046cf8ac8f7b0: Welcome.");
    output.assert_stdout_contains("selected choice 1cf4c868fe680ee4bb4c");

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
c8cced2046cf8ac8f7b0 = "1cf4c868fe680ee4bb4c"
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
    output.assert_stdout_contains("selected choice 1cf4c868fe680ee4bb4c");
}
