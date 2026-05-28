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
            "> intro\n",
            "  Hello.\n",
            "  ? help\n",
            "    Help me.\n",
            "    -> help\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> END\n",
            ":: help\n",
            "> helped\n",
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
            "msgctxt \"intro\"\n",
            "msgid \"Hello.\"\n",
            "msgstr \"Bonjour.\"\n",
            "\n",
            "msgctxt \"help\"\n",
            "msgid \"Help me.\"\n",
            "msgstr \"Aidez-moi.\"\n",
            "\n",
            "msgctxt \"helped\"\n",
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
intro = "help"
"#,
    );

    let default_fixture = write_file(
        temp.path(),
        "default.toml",
        r#"[choices]
intro = "help"
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
    default_run.assert_stdout_contains("prompt intro: Hello.");
    default_run.assert_stdout_contains("  [1] help: Help me.");
    default_run.assert_stdout_contains("line helped: Done.");

    let run_output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    run_output.assert_success().assert_stderr("");
    run_output.assert_stdout_contains("prompt intro: Bonjour.");
    run_output.assert_stdout_contains("  [1] help: Aidez-moi.");
    run_output.assert_stdout_contains("line helped: Done.");

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
        .write_all(b"help\n")
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait");
    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("prompt intro: Bonjour.");
    output.assert_stdout_contains("[1] help: Aidez-moi.");
    output.assert_stdout_contains("line helped: Done.");
}

#[test]
fn dialogue_locale_falls_back_to_language_catalog() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> intro\n  Hello.\n-> END\n",
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    write_file(
        temp.path(),
        "locale/fr.po",
        "msgctxt \"intro\"\nmsgid \"Hello.\"\nmsgstr \"Salut.\"\n",
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
    output.assert_stdout_contains("line intro: Salut.");

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
        ":: start default\n> intro\n  Hello.\n-> END\n",
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    write_file(
        temp.path(),
        "locale/zh-Hant.po",
        "msgctxt \"intro\"\nmsgid \"Hello.\"\nmsgstr \"Ni hao.\"\n",
    );
    write_file(
        temp.path(),
        "locale/zh.po",
        "msgctxt \"intro\"\nmsgid \"Hello.\"\nmsgstr \"Wrong fallback.\"\n",
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
    output.assert_stdout_contains("line intro: Ni hao.");

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
