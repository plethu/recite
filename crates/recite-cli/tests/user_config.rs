#![cfg(test)]

use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

mod support;
use support::*;

fn without_user_config(command: &mut Command, config_root: &Path) {
    command
        .env_remove("RECITE_CONFIG")
        .env("XDG_CONFIG_HOME", config_root)
        .env("HOME", config_root);
}

fn play_with_config(asset: &Path, config: &str, config_root: &Path) -> std::process::Output {
    let mut command = recite();
    command
        .arg("play")
        .arg(asset)
        .arg("--block")
        .arg("start")
        .arg("--ui")
        .arg("plain")
        .env("RECITE_CONFIG", config);
    command
        .env("XDG_CONFIG_HOME", config_root)
        .env("HOME", config_root);
    run(&mut command)
}

fn compile_without_user_config(root: &Path, source: &Path, output: &Path, config_root: &Path) {
    let source = source.strip_prefix(root).unwrap_or(source);
    let output = output.strip_prefix(root).unwrap_or(output);
    let mut command = recite();
    command
        .current_dir(root)
        .arg("compile")
        .arg("--output")
        .arg(output)
        .arg(source);
    without_user_config(&mut command, config_root);
    command.assert_success();
}

#[test]
fn versioned_and_legacy_configs_are_consumed_by_cli_play() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> intro@11111111111111111111\n  Hello.\n-> END\n",
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let config_root = temp.path().join("empty-config-root");

    for (name, config) in [
        (
            "versioned.toml",
            "config_version = 1\n[ui]\nkeymap = \"vim\"\nkey_hints = \"compact\"\ncolor = \"never\"\ncontrast = \"accessible\"\n[play]\nshow_unavailable_choices = false\n",
        ),
        ("legacy.toml", "[ui]\nkeymap = \"vim\"\n"),
    ] {
        let path = write_file(temp.path(), name, config);
        let output = play_with_config(&asset, &path.to_string_lossy(), &config_root);
        output.assert_success().assert_stderr("");
        output.assert_stdout_contains("line 11111111111111111111: Hello.");
    }
}

#[test]
fn explicit_config_failures_remain_typed_cli_errors() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> intro@11111111111111111111\n  Hello.\n-> END\n",
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let config_root = temp.path().join("empty-config-root");
    let missing = temp.path().join("missing.toml");
    let malformed = write_file(temp.path(), "malformed.toml", "[ui\nlocale =");
    let unsupported = write_file(temp.path(), "unsupported.toml", "config_version = 2\n");
    let invalid_locale = write_file(
        temp.path(),
        "invalid-locale.toml",
        "config_version = 1\n[ui]\nlocale = \"not a locale\"\n",
    );

    let cases: Vec<(String, String)> = vec![
        (String::new(), "RECITE_CONFIG is set but empty".to_owned()),
        (
            "relative.toml".to_owned(),
            "RECITE_CONFIG must be an absolute path".to_owned(),
        ),
        (
            missing.to_string_lossy().into_owned(),
            "explicit RECITE_CONFIG path does not exist".to_owned(),
        ),
        (
            temp.path().to_string_lossy().into_owned(),
            "failed to read UI config".to_owned(),
        ),
        (
            malformed.to_string_lossy().into_owned(),
            "failed to parse UI config".to_owned(),
        ),
        (
            unsupported.to_string_lossy().into_owned(),
            "unsupported version 2".to_owned(),
        ),
        (
            invalid_locale.to_string_lossy().into_owned(),
            "invalid [ui].locale".to_owned(),
        ),
    ];

    for (config, expected) in cases {
        let output = play_with_config(&asset, &config, &config_root);
        output.assert_failure().assert_exit_code(1);
        output.assert_stderr_contains(&expected);
    }
}

#[test]
fn malformed_user_config_does_not_change_compile_run_trace_or_dialogue_locale() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> intro@11111111111111111111\n  Welcome.\n-> END\n",
    );
    write_file(
        temp.path(),
        "locale/fr-FR.po",
        "msgctxt \"11111111111111111111\"\nmsgid \"Welcome.\"\nmsgstr \"Bienvenue.\"\n",
    );
    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        "[dialogue]\nlocale = \"fr-FR\"\n[dialogue.catalogs]\n\"fr-FR\" = [\"locale/fr-FR.po\"]\n",
    );
    let malformed = write_file(temp.path(), "malformed.toml", "[ui\nlocale =");
    let config_root = temp.path().join("empty-config-root");
    let baseline_asset = temp.path().join("baseline.recitec");
    compile_without_user_config(temp.path(), &source, &baseline_asset, &config_root);
    let baseline_asset_bytes = std::fs::read(&baseline_asset).expect("baseline asset");

    let mut configured_compile = recite();
    configured_compile
        .current_dir(temp.path())
        .arg("compile")
        .arg("--output")
        .arg("baseline.recitec")
        .arg(source.strip_prefix(temp.path()).unwrap_or(&source))
        .env("RECITE_CONFIG", &malformed)
        .env("XDG_CONFIG_HOME", &config_root)
        .env("HOME", &config_root);
    configured_compile.assert_success();
    assert_eq!(
        baseline_asset_bytes,
        std::fs::read(&baseline_asset).expect("configured asset"),
        "user UI config must not affect compiler output"
    );

    let mut baseline_run = recite();
    baseline_run
        .current_dir(temp.path())
        .arg("run")
        .arg(&baseline_asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture);
    without_user_config(&mut baseline_run, &config_root);
    let baseline_run = run(&mut baseline_run);
    baseline_run.assert_success().assert_stderr("");

    let mut configured_run = recite();
    configured_run
        .current_dir(temp.path())
        .arg("run")
        .arg(&baseline_asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture)
        .env("RECITE_CONFIG", &malformed)
        .env("XDG_CONFIG_HOME", &config_root)
        .env("HOME", &config_root);
    let configured_run = run(&mut configured_run);
    configured_run.assert_success().assert_stderr("");
    assert_eq!(baseline_run.stdout, configured_run.stdout);
    assert!(stdout(&configured_run).contains("line 11111111111111111111: Bienvenue."));

    let mut baseline_trace = recite();
    baseline_trace
        .current_dir(temp.path())
        .arg("trace")
        .arg(&baseline_asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture);
    without_user_config(&mut baseline_trace, &config_root);
    let baseline_trace = run(&mut baseline_trace);
    baseline_trace.assert_success().assert_stderr("");

    let mut configured_trace = recite();
    configured_trace
        .current_dir(temp.path())
        .arg("trace")
        .arg(&baseline_asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture)
        .env("RECITE_CONFIG", &malformed)
        .env("XDG_CONFIG_HOME", &config_root)
        .env("HOME", &config_root);
    let configured_trace = run(&mut configured_trace);
    configured_trace.assert_success().assert_stderr("");
    assert_eq!(baseline_trace.stdout, configured_trace.stdout);
    let trace: serde_json::Value = serde_json::from_slice(&configured_trace.stdout).expect("trace");
    assert_eq!(trace["events"][0]["line"]["text"], "Bienvenue.");
}

#[test]
fn help_falls_back_without_stderr_when_config_loading_fails() {
    let temp = TempDir::new().expect("tempdir");
    let malformed = write_file(temp.path(), "malformed.toml", "[ui\nlocale =");

    let mut command = recite();
    command
        .arg("--help")
        .env("RECITE_CONFIG", malformed)
        .env("XDG_CONFIG_HOME", temp.path())
        .env("HOME", temp.path());
    let output = run(&mut command);
    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("Recite dialogue compiler and validation CLI.");
}

#[test]
fn play_keymap_invocation_is_checked_by_shared_resolution_adapter() {
    let loaded = recite_config::LoadedUserConfig::from_explicit(recite_config::UserConfig {
        config_version: recite_config::CONFIG_VERSION,
        ui: recite_config::UiConfig {
            locale: recite_config::UiLocale::default(),
            keymap: recite_config::Keymap::Vim,
            key_hints: recite_config::KeyHints::default(),
            color: recite_config::TuiColorMode::default(),
            contrast: recite_config::TuiContrast::default(),
        },
        play: recite_config::PlayConfig::default(),
    });
    let resolved = recite_config::resolve_user_config(
        &loaded,
        &recite_config::InvocationOverrides::new().with_keymap(recite_config::Keymap::Standard),
    );
    assert_eq!(
        resolved.ui().keymap().value(),
        &recite_config::Keymap::Standard
    );
}
