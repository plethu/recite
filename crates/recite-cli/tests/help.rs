#![cfg(test)]

mod support;
use support::*;

use tempfile::TempDir;

#[test]
fn help_covers_issue_25_commands_and_options() {
    let output = run(recite().arg("--help"));
    output.assert_success().assert_stderr("");
    for command in [
        "validate", "compile", "extract", "id", "project", "schema", "explain", "watch", "run",
        "trace", "play", "bench",
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

    let validate = run(recite().arg("validate").arg("--help"));
    validate.assert_success().assert_stderr("");
    validate.assert_stdout_contains("--schema <SCHEMA>");
    validate.assert_stdout_contains("--only <CATEGORY>");
    validate.assert_stdout_contains("[possible values: markup, metadata]");

    let id = run(recite().args(["id", "--help"]));
    id.assert_success().assert_stderr("");
    id.assert_stdout_contains("Usage: recite id <COMMAND>");
    id.assert_stdout_contains("check");
    id.assert_stdout_contains("assign");

    let check_ids = run(recite().args(["id", "check", "--help"]));
    check_ids.assert_success().assert_stderr("");
    check_ids.assert_stdout_contains("--since <REF>");
    check_ids.assert_stdout_contains("--allow-removed <ID>");

    let assign_ids = run(recite().args(["id", "assign", "--help"]));
    assign_ids.assert_success().assert_stderr("");
    assign_ids.assert_stdout_contains("Append generated stable IDs");

    let project = run(recite().args(["project", "validate", "--help"]));
    project.assert_success().assert_stderr("");
    project.assert_stdout_contains("recite.project.toml");

    let fresh = run(recite().args(["project", "check-fresh", "--help"]));
    fresh.assert_success().assert_stderr("");
    fresh.assert_stdout_contains("compiled assets are fresh");

    let inspect = run(recite().args(["schema", "inspect", "--help"]));
    inspect.assert_success().assert_stderr("");
    inspect.assert_stdout_contains("Usage: recite schema inspect <SCHEMA>");
    inspect.assert_stdout_contains("deterministic machine-readable JSON");

    let export = run(recite().args(["schema", "export", "--help"]));
    export.assert_success().assert_stderr("");
    export.assert_stdout_contains("--schema <SCHEMA>");
    export.assert_stdout_contains("--output <OUTPUT>");
    export.assert_stdout_contains("validated canonical schema manifest");

    let explain = run(recite().arg("explain").arg("--help"));
    explain.assert_success().assert_stderr("");
    explain.assert_stdout_contains("Usage: recite explain <CODE>");
    assert!(stdout(&explain).contains("Arguments:\n  <CODE>"));
    explain.assert_stdout_contains("Stable diagnostic code");

    let watch = run(recite().arg("watch").arg("--help"));
    watch.assert_success().assert_stderr("");
    watch.assert_stdout_contains("Usage: recite watch [OPTIONS] <PROJECT_ROOT>");
    watch.assert_stdout_contains("--output-format <OUTPUT_FORMAT>");
    watch.assert_stdout_contains("--invocation-id <INVOCATION_ID>");
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
    top.assert_stdout_contains("schema");
    top.assert_stdout_contains("-h, --help");
    // en-GB is a test-only fixture, not a launch-locale support claim. The
    // incomplete fixture therefore falls back to the human-authored en-US UI.
    top.assert_stdout_contains("Print help");

    let compile = run(recite()
        .arg("compile")
        .arg("--help")
        .env("RECITE_CONFIG", &config));
    compile.assert_success().assert_stderr("");
    compile.assert_stdout_contains("Usage: recite compile [OPTIONS] --output <OUTPUT> <PATHS>...");
    compile.assert_stdout_contains("Compile dialogue source to a MessagePack .recitec asset");
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

    let bench = run(recite()
        .arg("bench")
        .arg("--help")
        .env("RECITE_CONFIG", &config));
    bench.assert_success().assert_stderr("");
    bench.assert_stdout_contains("Generate benchmark scale reports for fixtures or a project root");

    let help_compile = run(recite()
        .arg("help")
        .arg("compile")
        .env("RECITE_CONFIG", &config));
    help_compile.assert_success().assert_stderr("");
    help_compile.assert_stdout_contains("Compile dialogue source to a MessagePack .recitec asset");
    help_compile.assert_stdout_contains("Print help");
}

#[test]
fn nested_help_follows_the_ui_locale_config_and_falls_back_to_the_default_catalog() {
    let temp = TempDir::new().expect("tempdir");
    let config = write_file(
        temp.path(),
        "config.toml",
        r#"[ui]
locale = "en-GB"
"#,
    );

    // The en-GB fixture is incomplete, so every nested message falls back to en-US.
    let group = run(recite()
        .args(["id", "--help"])
        .env("RECITE_CONFIG", &config));
    group.assert_success().assert_stderr("");
    group.assert_stdout_contains("Check and assign stable line and choice IDs");
    group.assert_stdout_contains("Append generated stable IDs");
    group.assert_stdout_contains("Print help");

    let leaf = run(recite()
        .args(["schema", "check-fresh", "--help"])
        .env("RECITE_CONFIG", &config));
    leaf.assert_success().assert_stderr("");
    leaf.assert_stdout_contains("Compare current schema producer fingerprints");
}

#[test]
fn every_command_leaf_has_localised_help() {
    // A command whose `about` is missing from the catalogue renders clap's English fallback or
    // nothing. Walk the whole tree through the real binary so new nouns cannot skip localisation.
    let top = run(recite().arg("--help"));
    for noun in ["id", "project", "schema"] {
        let group = run(recite().args([noun, "--help"]));
        group.assert_success().assert_stderr("");
        let text = stdout(&group);
        let first = text.lines().next().unwrap_or_default();
        assert!(!first.is_empty(), "`recite {noun}` has no about");
        assert!(
            !text.contains("Print this message or the help of the given subcommand"),
            "`recite {noun}` shows clap's English help subcommand"
        );
        let verbs = text
            .lines()
            .skip_while(|line| *line != "Commands:")
            .skip(1)
            .take_while(|line| !line.is_empty())
            .filter_map(|line| line.split_whitespace().next())
            .collect::<Vec<_>>();
        assert!(!verbs.is_empty(), "`recite {noun}` lists no commands");
        for verb in verbs {
            let leaf = run(recite().args([noun, verb, "--help"]));
            leaf.assert_success().assert_stderr("");
            let leaf_text = stdout(&leaf);
            let about = leaf_text.lines().next().unwrap_or_default();
            assert!(
                about.len() > 10 && !about.starts_with("Usage"),
                "`recite {noun} {verb}` has no localised about: {about:?}"
            );
        }
    }
    top.assert_success();
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
    assert!(
        stdout(&output)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains("Compile dialogue source to a MessagePack .recitec asset")
    );

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
    assert!(
        stdout(&output)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains("Compile dialogue source to a MessagePack .recitec asset")
    );

    let output = run(recite().arg("--help").env("RECITE_CONFIG", temp.path()));
    output.assert_success().assert_stderr("");
    assert!(
        stdout(&output)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains("Compile dialogue source to a MessagePack .recitec asset")
    );
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
            "> @intro\n",
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
