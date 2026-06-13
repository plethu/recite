#![cfg(test)]

mod support;
use support::*;

use tempfile::TempDir;

#[test]
fn help_covers_issue_25_commands_and_options() {
    let output = run(recite().arg("--help"));
    output.assert_success().assert_stderr("");
    for command in [
        "validate",
        "compile",
        "extract",
        "check-ids",
        "check-markup",
        "check-metadata",
        "validate-project",
        "check-fresh",
        "explain",
        "watch",
        "run",
        "trace",
        "play",
    ] {
        output.assert_stdout_contains(command);
    }

    let compile = run(recite().arg("compile").arg("--help"));
    compile.assert_success().assert_stderr("");
    compile.assert_stdout_contains("Usage: recite compile [OPTIONS] --output <OUTPUT> <PATHS>...");
    compile.assert_stdout_contains("--schema <SCHEMA>");

    let extract = run(recite().arg("extract").arg("--help"));
    extract.assert_success().assert_stderr("");
    extract.assert_stdout_contains("--output <OUTPUT>");
    extract.assert_stdout_contains("--schema <SCHEMA>");

    let metadata = run(recite().arg("check-metadata").arg("--help"));
    metadata.assert_success().assert_stderr("");
    metadata.assert_stdout_contains("--schema <SCHEMA>");

    let project = run(recite().arg("validate-project").arg("--help"));
    project.assert_success().assert_stderr("");
    project.assert_stdout_contains("recite.project.toml");

    let fresh = run(recite().arg("check-fresh").arg("--help"));
    fresh.assert_success().assert_stderr("");
    fresh.assert_stdout_contains("compiled assets are fresh");

    let explain = run(recite().arg("explain").arg("--help"));
    explain.assert_success().assert_stderr("");
    explain.assert_stdout_contains("Usage: recite explain <CODE>");
    assert!(stdout(&explain).contains("Arguments:\n  <CODE>"));
    explain.assert_stdout_contains("Stable diagnostic code");

    let watch = run(recite().arg("watch").arg("--help"));
    watch.assert_success().assert_stderr("");
    watch.assert_stdout_contains("Usage: recite watch <PROJECT_ROOT>");
    watch.assert_stdout_contains("recite.project.toml");

    let run_help = run(recite().arg("run").arg("--help"));
    run_help.assert_success().assert_stderr("");
    run_help.assert_stdout_contains("--block <BLOCK>");
    run_help.assert_stdout_contains("--fixture <FIXTURE>");
    assert!(!stdout(&run_help).contains("--metrics"));

    let trace = run(recite().arg("trace").arg("--help"));
    trace.assert_success().assert_stderr("");
    trace.assert_stdout_contains("--block <BLOCK>");
    trace.assert_stdout_contains("--fixture <FIXTURE>");
    trace.assert_stdout_contains("--metrics");

    let play = run(recite().arg("play").arg("--help"));
    play.assert_success().assert_stderr("");
    play.assert_stdout_contains("--block <BLOCK>");
    play.assert_stdout_contains("--ui <UI>");
    play.assert_stdout_contains("--keymap <KEYMAP>");
    play.assert_stdout_contains("--dialogue-locale <DIALOGUE_LOCALE>");
    play.assert_stdout_contains("--dialogue-catalog <DIALOGUE_CATALOG>");
}

#[test]
fn help_uses_ui_locale_config_for_recite_owned_text() {
    let temp = TempDir::new().expect("tempdir");
    let config = write_file(
        temp.path(),
        "config.toml",
        r#"[ui]
locale = "en-GB"
"#,
    );

    let top = run(recite().arg("--help").env("RECITE_CONFIG", &config));
    top.assert_success().assert_stderr("");
    top.assert_stdout_contains("compile");
    top.assert_stdout_contains("check-metadata");
    top.assert_stdout_contains("-h, --help");
    top.assert_stdout_contains("Show help");

    let compile = run(recite()
        .arg("compile")
        .arg("--help")
        .env("RECITE_CONFIG", &config));
    compile.assert_success().assert_stderr("");
    compile.assert_stdout_contains("Usage: recite compile [OPTIONS] --output <OUTPUT> <PATHS>...");
    compile.assert_stdout_contains("Compile dialogue source to a MessagePack .recitec artefact");
    compile.assert_stdout_contains("--output <OUTPUT>");
    compile.assert_stdout_contains("--schema <SCHEMA>");

    let play = run(recite()
        .arg("play")
        .arg("--help")
        .env("RECITE_CONFIG", &config));
    play.assert_success().assert_stderr("");
    play.assert_stdout_contains("Play a compiled asset interactively");
    play.assert_stdout_contains("--block <BLOCK>");
    play.assert_stdout_contains("--ui <UI>");
    play.assert_stdout_contains("--dialogue-locale <DIALOGUE_LOCALE>");
    play.assert_stdout_contains("-h, --help");

    let help_compile = run(recite()
        .arg("help")
        .arg("compile")
        .env("RECITE_CONFIG", &config));
    help_compile.assert_success().assert_stderr("");
    help_compile
        .assert_stdout_contains("Compile dialogue source to a MessagePack .recitec artefact");
    help_compile.assert_stdout_contains("Show help");
}

#[test]
fn help_falls_back_to_default_catalog_for_missing_locale_messages() {
    let temp = TempDir::new().expect("tempdir");
    let config = write_file(
        temp.path(),
        "config.toml",
        r#"[ui]
locale = "en-GB"
"#,
    );

    let compile = run(recite()
        .arg("compile")
        .arg("--help")
        .env("RECITE_CONFIG", &config));
    compile.assert_success().assert_stderr("");
    compile.assert_stdout_contains("Generated schema manifest JSON");
}

#[test]
fn help_ignores_malformed_ui_config() {
    let temp = TempDir::new().expect("tempdir");
    let malformed_config = write_file(temp.path(), "malformed.toml", "[ui\nlocale =");

    let output = run(recite()
        .arg("--help")
        .env("RECITE_CONFIG", &malformed_config));
    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("Recite dialogue compiler and validation CLI.");
    output.assert_stdout_contains("Compile dialogue source to a MessagePack .recitec asset");

    let invalid_locale_config = write_file(
        temp.path(),
        "invalid-locale.toml",
        r#"[ui]
locale = "not a locale"
"#,
    );
    let output = run(recite()
        .arg("--help")
        .env("RECITE_CONFIG", &invalid_locale_config));
    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("Compile dialogue source to a MessagePack .recitec asset");

    let output = run(recite().arg("--help").env("RECITE_CONFIG", temp.path()));
    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("Compile dialogue source to a MessagePack .recitec asset");
}

#[test]
fn ui_locale_does_not_translate_trace_machine_output() {
    let temp = TempDir::new().expect("tempdir");
    let config = write_file(
        temp.path(),
        "config.toml",
        r#"[ui]
locale = "en-GB"
"#,
    );
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> intro@9ec4d9d645646f47c553\n",
            "  Welcome.\n",
            "-> END\n"
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let fixture = write_file(temp.path(), "fixture.toml", "");

    let output = run(recite()
        .arg("trace")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture)
        .env("RECITE_CONFIG", &config));

    output.assert_success().assert_stderr("");
    let trace: serde_json::Value = serde_json::from_slice(&output.stdout).expect("trace is JSON");
    assert_eq!(trace["block"], "start");
    assert!(trace.get("final_deferred_effects").is_some());
    assert!(
        trace["events"]
            .as_array()
            .expect("events array")
            .iter()
            .any(|event| event["type"] == "line" && event["line"]["text"] == "Welcome.")
    );
}
