use std::path::{Path, PathBuf};

use tempfile::TempDir;

mod support;
use support::*;

fn simple_asset(temp: &TempDir) -> PathBuf {
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> intro\n  Hello.\n-> END\n",
    );
    compile_project_asset(temp.path(), &source, "dialogue.recitec", None)
}

fn run_fixture(asset: &Path, fixture: &Path) -> std::process::Output {
    run(recite()
        .arg("run")
        .arg(asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(fixture))
}

#[test]
fn catalog_without_dialogue_locale_is_rejected() {
    let temp = TempDir::new().expect("tempdir");
    let asset = simple_asset(&temp);
    let fixture = write_file(
        temp.path(),
        "missing-locale.toml",
        r#"[dialogue.catalogs]
"fr-FR" = ["locale/fr-FR.po"]
"#,
    );

    let output = run_fixture(&asset, &fixture);
    output.assert_failure();
    output.assert_stderr_contains("dialogue catalogs require a dialogue locale");

    let output = run(recite()
        .arg("play")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--ui")
        .arg("plain")
        .arg("--dialogue-catalog")
        .arg("fr-FR=missing.po"));
    output.assert_failure();
    output.assert_stderr_contains("dialogue catalogs require a dialogue locale");
}

#[test]
fn malformed_catalog_reports_catalog_path_and_line() {
    let temp = TempDir::new().expect("tempdir");
    let asset = simple_asset(&temp);
    let malformed_catalog = write_file(
        temp.path(),
        "malformed.po",
        "msgctxt \"intro\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\n",
    );
    let fixture = write_file(
        temp.path(),
        "malformed.toml",
        &format!(
            "[dialogue]\nlocale = \"fr-FR\"\n\n[dialogue.catalogs]\n\"fr-FR\" = [\"{}\"]\n",
            malformed_catalog.display()
        ),
    );

    let output = run(recite()
        .arg("trace")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    output.assert_failure();
    output.assert_stderr_contains("failed to parse dialogue catalog");
    output.assert_stderr_contains("unterminated quoted string");
}

#[test]
fn conflicting_catalog_entries_are_rejected() {
    let temp = TempDir::new().expect("tempdir");
    let asset = simple_asset(&temp);
    let first = write_file(
        temp.path(),
        "first.po",
        "msgctxt \"intro\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n",
    );
    let second = write_file(
        temp.path(),
        "second.po",
        "msgctxt \"intro\"\nmsgid \"Hello.\"\nmsgstr \"Salut.\"\n",
    );
    let fixture = write_file(
        temp.path(),
        "conflict.toml",
        &format!(
            "[dialogue]\nlocale = \"fr-FR\"\n\n[dialogue.catalogs]\n\"fr-FR\" = [\"{}\", \"{}\"]\n",
            first.display(),
            second.display()
        ),
    );

    let output = run_fixture(&asset, &fixture);
    output.assert_failure();
    output.assert_stderr_contains("conflicting translations");
}

#[test]
fn invalid_catalog_specs_and_locales_are_rejected() {
    let temp = TempDir::new().expect("tempdir");
    let asset = simple_asset(&temp);

    let output = run(recite()
        .arg("play")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--ui")
        .arg("plain")
        .arg("--dialogue-locale")
        .arg("fr-FR")
        .arg("--dialogue-catalog")
        .arg("fr-FR"));
    output.assert_failure();
    output.assert_stderr_contains("expected LOCALE=PATH");

    let output = run(recite()
        .arg("play")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--ui")
        .arg("plain")
        .arg("--dialogue-locale")
        .arg("not a locale"));
    output.assert_failure();
    output.assert_stderr_contains("invalid dialogue locale in --dialogue-locale");

    let fixture = write_file(
        temp.path(),
        "invalid-locale.toml",
        "[dialogue]\nlocale = \"not a locale\"\n",
    );
    let output = run_fixture(&asset, &fixture);
    output.assert_failure();
    output.assert_stderr_contains("invalid dialogue locale in [dialogue].locale");
}

#[test]
fn plural_catalog_entries_are_rejected() {
    let temp = TempDir::new().expect("tempdir");
    let asset = simple_asset(&temp);
    let plural_catalog = write_file(
        temp.path(),
        "plural.po",
        concat!(
            "msgctxt \"intro\"\n",
            "msgid \"Hello.\"\n",
            "msgid_plural \"Hello.\"\n",
            "msgstr[0] \"Bonjour.\"\n",
        ),
    );
    let fixture = write_file(
        temp.path(),
        "plural.toml",
        &format!(
            "[dialogue]\nlocale = \"fr-FR\"\n\n[dialogue.catalogs]\n\"fr-FR\" = [\"{}\"]\n",
            plural_catalog.display()
        ),
    );

    let output = run(recite()
        .arg("trace")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    output.assert_failure();
    output.assert_stderr_contains("plural entries are not supported");
}
