#![cfg(test)]

use std::io::Write;
use std::process::Stdio;

use tempfile::TempDir;

mod support;
use support::*;

#[test]
fn run_trace_and_play_plain_preview_dialogue_locale_catalogs() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> intro@75f6c2ec830c44fc0b07\n",
            "  Hello.\n",
            "  ? help@7648ae75984a9367d2b8\n",
            "    Help me.\n",
            "    -> help\n",
            "  ? leave@d095be3772cca8f41dfc\n",
            "    Leave.\n",
            "    -> END\n",
            ":: help\n",
            "> helped@a65d329b1d6d610af8ee\n",
            "  Done.\n",
            "-> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let catalog = write_file(
        temp.path(),
        "locale/fr-FR.po",
        concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: fr-FR\\n\"\n",
            "\n",
            "msgctxt \"75f6c2ec830c44fc0b07\"\n",
            "msgid \"Hello.\"\n",
            "msgstr \"Bonjour.\"\n",
            "\n",
            "msgctxt \"7648ae75984a9367d2b8\"\n",
            "msgid \"Help me.\"\n",
            "msgstr \"Aidez-moi.\"\n",
            "\n",
            "msgctxt \"a65d329b1d6d610af8ee\"\n",
            "msgid \"Done.\"\n",
            "msgstr \"\"\n",
        ),
    );
    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        r#"[dialogue]
locale = "fr-FR"

[dialogue.catalogs]
"fr-FR" = ["locale/fr-FR.po"]

[choices]
"75f6c2ec830c44fc0b07" = "7648ae75984a9367d2b8"
"#,
    );

    let default_fixture = write_file(
        temp.path(),
        "default.toml",
        r#"[choices]
"75f6c2ec830c44fc0b07" = "7648ae75984a9367d2b8"
"#,
    );
    let default_run = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&default_fixture));
    default_run.assert_success().assert_stderr("");
    default_run.assert_stdout_contains("prompt 75f6c2ec830c44fc0b07: Hello.");
    default_run.assert_stdout_contains("  [1] 7648ae75984a9367d2b8: Help me.");
    default_run.assert_stdout_contains("line a65d329b1d6d610af8ee: Done.");

    let run_output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    run_output.assert_success().assert_stderr("");
    run_output.assert_stdout_contains("prompt 75f6c2ec830c44fc0b07: Bonjour.");
    run_output.assert_stdout_contains("  [1] 7648ae75984a9367d2b8: Aidez-moi.");
    run_output.assert_stdout_contains("line a65d329b1d6d610af8ee: Done.");

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
    assert_eq!(trace["dialogue_locale"], "fr-FR");
    assert_eq!(
        trace["dialogue_locale_fallbacks"],
        serde_json::json!(["fr-FR", "fr"])
    );
    let events = trace["events"].as_array().expect("events array");
    let prompt = events
        .iter()
        .find(|event| event["type"] == "prompt")
        .expect("prompt event");
    assert_eq!(prompt["prompt"]["line"]["source_text"], "Hello.");
    assert_eq!(prompt["prompt"]["line"]["text"], "Bonjour.");
    assert_eq!(prompt["prompt"]["choices"][0]["source_text"], "Help me.");
    assert_eq!(prompt["prompt"]["choices"][0]["text"], "Aidez-moi.");
    assert!(prompt["prompt"]["identity"]["fixture_keys"].is_array());

    let default_trace = run(recite()
        .arg("trace")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&default_fixture));
    default_trace.assert_success().assert_stderr("");
    let trace: serde_json::Value =
        serde_json::from_slice(&default_trace.stdout).expect("trace is JSON");
    assert!(trace.get("dialogue_locale").is_none());

    let metrics_trace = run(recite()
        .arg("trace")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture)
        .arg("--metrics"));
    metrics_trace.assert_success().assert_stderr("");
    let trace: serde_json::Value =
        serde_json::from_slice(&metrics_trace.stdout).expect("trace is JSON");
    assert_eq!(trace["metrics"]["localization_lookup_count"], 4);

    let mut child = recite()
        .arg("play")
        .arg("dialogue.recitec")
        .arg("--block")
        .arg("start")
        .arg("--ui")
        .arg("plain")
        .arg("--dialogue-locale")
        .arg("fr-FR")
        .arg("--dialogue-catalog")
        .arg(format!(
            "fr-FR={}",
            catalog.strip_prefix(temp.path()).unwrap().display()
        ))
        .current_dir(temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn recite play");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"7648ae75984a9367d2b8\n")
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");
    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("prompt 75f6c2ec830c44fc0b07: Bonjour.");
    output.assert_stdout_contains("[1] 7648ae75984a9367d2b8: Aidez-moi.");
    output.assert_stdout_contains("line a65d329b1d6d610af8ee: Done.");
}

#[test]
fn dialogue_locale_falls_back_to_language_catalog() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> intro@11111111111111111111\n  Hello.\n-> END\n",
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    write_file(
        temp.path(),
        "locale/fr.po",
        "msgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"Salut.\"\n",
    );
    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        r#"[dialogue]
locale = "fr-FR"

[dialogue.catalogs]
fr = ["locale/fr.po"]
"#,
    );

    let output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("line 11111111111111111111: Salut.");

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
    assert_eq!(
        trace["dialogue_locale_fallbacks"],
        serde_json::json!(["fr-FR", "fr"])
    );
}

#[test]
fn dialogue_locale_falls_back_through_intermediate_locale() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> intro@11111111111111111111\n  Hello.\n-> END\n",
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    write_file(
        temp.path(),
        "locale/zh-Hant.po",
        "msgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"Ni hao.\"\n",
    );
    write_file(
        temp.path(),
        "locale/zh.po",
        "msgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"Wrong fallback.\"\n",
    );
    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        r#"[dialogue]
locale = "zh-Hant-TW"

[dialogue.catalogs]
"zh-Hant" = ["locale/zh-Hant.po"]
zh = ["locale/zh.po"]
"#,
    );

    let output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("line 11111111111111111111: Ni hao.");

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
    assert_eq!(
        trace["dialogue_locale_fallbacks"],
        serde_json::json!(["zh-Hant-TW", "zh-Hant", "zh"])
    );
}
