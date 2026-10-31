#![cfg(test)]

use std::io::Write;
use std::process::Stdio;

use tempfile::TempDir;

mod support;
use support::*;

#[path = "dialogue_locale/shared_pressure.rs"]
mod shared_pressure;

#[test]
fn run_trace_and_play_plain_preview_dialogue_locale_catalogs() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> @intro\n",
            "  Hello.\n",
            "  ? @help\n",
            "    Help me.\n",
            "    -> help\n",
            "  ? @leave\n",
            "    Leave.\n",
            "    -> END\n",
            ":: help\n",
            "> @helped\n",
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
"intro" = "help"
"#,
    );

    let default_fixture = write_file(
        temp.path(),
        "default.toml",
        r#"[choices]
"intro" = "help"
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
    let structured = run(recite()
        .args([
            "trace",
            "--output-format",
            "structured",
            "--invocation-id",
            "locale-trace",
        ])
        .arg(&asset)
        .args(["--block", "start", "--fixture"])
        .arg(&fixture));
    structured.assert_success().assert_stderr("");
    let records: Vec<serde_json::Value> = String::from_utf8_lossy(&structured.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).expect("structured trace record is JSON"))
        .collect();
    assert_eq!(records.len(), 2);
    assert_eq!(records[1]["event"], "command.result");
    assert_eq!(records[1]["invocation_id"], "locale-trace");
    assert_eq!(records[1]["data"]["trace"]["dialogue_locale"], "fr-FR");

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
        ":: start default\n> @ffee6p8\n  Hello.\n-> END\n",
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    write_file(
        temp.path(),
        "locale/fr.po",
        include_str!("../../../fixtures/recite/valid/locale_fallback_fr.po"),
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
    output.assert_stdout_contains("line ffee6p8: Salut.");

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
        ":: start default\n> @ffee6p8\n  Hello.\n-> END\n",
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    write_file(
        temp.path(),
        "locale/zh-Hant.po",
        "msgctxt \"ffee6p8\"\nmsgid \"Hello.\"\nmsgstr \"Ni hao.\"\n",
    );
    write_file(
        temp.path(),
        "locale/zh.po",
        "msgctxt \"ffee6p8\"\nmsgid \"Hello.\"\nmsgstr \"Wrong fallback.\"\n",
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
    output.assert_stdout_contains("line ffee6p8: Ni hao.");

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

#[test]
fn plural_dialogue_uses_gettext_arms_and_records_resolution_trace() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> bind=(count:int=$remaining) @rthh7ab\n",
            "  You have one letter.\n",
            "  | You have {count} letters.\n",
            "-> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    write_file(
        temp.path(),
        "locale/fr.po",
        concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: fr\\n\"\n",
            "\"Plural-Forms: nplurals=3; plural=(n == 0 ? 0 : n == 1 ? 1 : 2);\\n\"\n",
            "\n",
            "msgctxt \"rthh7ab\"\n",
            "msgid \"You have one letter.\"\n",
            "msgid_plural \"You have {count} letters.\"\n",
            "msgstr[0] \"Vous avez une lettre.\"\n",
            "msgstr[1] \"Vous avez {count} lettre.\"\n",
            "msgstr[2] \"Vous avez {count} lettres.\"\n",
        ),
    );
    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        r#"[dialogue]
locale = "fr-CA"

[dialogue.catalogs]
fr = ["locale/fr.po"]

[interpolation_values]
remaining = { int = 2 }
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
    run_output.assert_stdout_contains("line rthh7ab: Vous avez 2 lettres.");

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
        trace["events"][0]["line"],
        serde_json::json!({
            "id": "rthh7ab",
            "source_text": "You have {count} letters.",
            "text": "Vous avez 2 lettres.",
            "speaker": null,
            "metadata": [],
            "plural": {
                "singular_source_text": "You have one letter.",
                "plural_source_text": "You have {count} letters.",
                "count": 2,
                "selected_arm": 2,
                "attempts": [
                    {
                        "locale": "fr-CA",
                        "context": "rthh7ab",
                        "key": "rthh7ab",
                        "selected_arm": null,
                        "outcome": "missing_plural_forms"
                    },
                    {
                        "locale": "fr",
                        "context": "rthh7ab",
                        "key": "rthh7ab",
                        "selected_arm": 2,
                        "outcome": "matched"
                    }
                ],
                "matched_locale": "fr",
                "matched_context": "rthh7ab",
                "matched_key": "rthh7ab",
                "matched_arm": 2,
                "source_fallback_arm": null,
                "outcome": "translated"
            }
        })
    );
}

#[test]
fn plural_trace_records_no_match_and_all_empty_source_fallbacks() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> bind=(count:int=$remaining) @bqyvpqq\n",
            "  One missing letter.\n",
            "  | Many missing letters.\n",
            "> bind=(count:int=$remaining) @z0g6c80\n",
            "  One empty letter.\n",
            "  | Many empty letters.\n",
            "-> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    write_file(
        temp.path(),
        "locale/fr.po",
        concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: fr\\n\"\n",
            "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n",
            "msgctxt \"z0g6c80\"\n",
            "msgid \"One empty letter.\"\n",
            "msgid_plural \"Many empty letters.\"\n",
            "msgstr[0] \"\"\n",
            "msgstr[1] \"\"\n",
        ),
    );
    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        r#"[dialogue]
locale = "fr"

[dialogue.catalogs]
fr = ["locale/fr.po"]

[interpolation_values]
remaining = { int = 2 }
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
    let events = trace["events"].as_array().expect("events array");
    let missing = events
        .iter()
        .find(|event| event["line"]["id"] == "bqyvpqq")
        .expect("missing-entry line trace");
    assert_eq!(missing["line"]["text"], "Many missing letters.");
    assert_eq!(missing["line"]["plural"]["source_fallback_arm"], 1);
    assert_eq!(
        missing["line"]["plural"]["attempts"][0]["outcome"],
        "missing_entry"
    );

    let empty = events
        .iter()
        .find(|event| event["line"]["id"] == "z0g6c80")
        .expect("all-empty line trace");
    assert_eq!(empty["line"]["text"], "Many empty letters.");
    assert_eq!(empty["line"]["plural"]["source_fallback_arm"], 1);
    assert_eq!(
        empty["line"]["plural"]["attempts"][0]["outcome"],
        "missing_translation"
    );
}
