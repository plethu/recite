#![cfg(test)]

use std::path::{Path, PathBuf};

use tempfile::TempDir;

mod support;
use support::*;

fn simple_asset(temp: &TempDir) -> PathBuf {
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> intro@11111111111111111111\n  Hello.\n-> END\n",
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
        "msgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\n",
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
        "msgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n",
    );
    let second = write_file(
        temp.path(),
        "second.po",
        "msgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"Salut.\"\n",
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
fn runtime_projection_ignores_fuzzy_and_obsolete_entries() {
    let temp = TempDir::new().expect("tempdir");
    let asset = simple_asset(&temp);
    let catalog = write_file(
        temp.path(),
        "stale.po",
        concat!(
            "msgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n\n",
            "#, fuzzy\nmsgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"Fuzzy.\"\n\n",
            "#~ msgctxt \"11111111111111111111\"\n#~ msgid \"Hello.\"\n#~ msgstr \"Obsolete.\"\n",
        ),
    );
    let fixture = write_file(
        temp.path(),
        "stale.toml",
        &format!(
            "[dialogue]\nlocale = \"fr-FR\"\n\n[dialogue.catalogs]\n\"fr-FR\" = [\"{}\"]\n",
            catalog.display()
        ),
    );
    let output = run_fixture(&asset, &fixture);
    output.assert_success();
    output.assert_stdout_contains("line 11111111111111111111: Bonjour.");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("Fuzzy."));
    assert!(!stdout.contains("Obsolete."));
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
fn plural_catalog_entries_require_complete_plural_metadata() {
    let temp = TempDir::new().expect("tempdir");
    let asset = simple_asset(&temp);
    let plural_catalog = write_file(
        temp.path(),
        "plural.po",
        concat!(
            "msgctxt \"11111111111111111111\"\n",
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
    output.assert_stderr_contains("active plural entries require Plural-Forms");
}

#[test]
fn unusable_plural_rules_are_rejected_before_runtime_lookup() {
    let temp = TempDir::new().expect("tempdir");
    let asset = simple_asset(&temp);
    let catalog = write_file(
        temp.path(),
        "unusable-plural.po",
        concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Plural-Forms: nplurals=2; plural=2;\\n\"\n\n",
            "msgctxt \"11111111111111111111\"\n",
            "msgid \"Hello.\"\n",
            "msgid_plural \"Hellos.\"\n",
            "msgstr[0] \"Bonjour.\"\n",
            "msgstr[1] \"Bonsjours.\"\n",
        ),
    );
    let fixture = write_file(
        temp.path(),
        "unusable-plural.toml",
        &format!(
            "[dialogue]\nlocale = \"fr-FR\"\n\n[dialogue.catalogs]\n\"fr-FR\" = [\"{}\"]\n",
            catalog.display()
        ),
    );

    let output = run_fixture(&asset, &fixture);
    output.assert_failure();
    output.assert_stderr_contains("invalid PO Plural-Forms rule");
    output.assert_stderr_contains("selected arm 2, but nplurals is 2");
}

#[test]
fn unusable_plural_arithmetic_is_projected_as_a_catalog_rule_error() {
    let temp = TempDir::new().expect("tempdir");
    let asset = simple_asset(&temp);
    let catalog = write_file(
        temp.path(),
        "division-by-zero.po",
        concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Plural-Forms: nplurals=2; plural=n / 0;\\n\"\n\n",
            "msgctxt \"11111111111111111111\"\n",
            "msgid \"Hello.\"\n",
            "msgid_plural \"Hellos.\"\n",
            "msgstr[0] \"Bonjour.\"\n",
            "msgstr[1] \"Bonsjours.\"\n",
        ),
    );
    let fixture = write_file(
        temp.path(),
        "division-by-zero.toml",
        &format!(
            "[dialogue]\nlocale = \"fr-FR\"\n\n[dialogue.catalogs]\n\"fr-FR\" = [\"{}\"]\n",
            catalog.display()
        ),
    );

    let output = run_fixture(&asset, &fixture);
    output.assert_failure();
    output.assert_stderr_contains("invalid PO Plural-Forms rule");
    output.assert_stderr_contains("plural expression divided by zero");
}

#[test]
fn non_empty_catalog_translations_must_preserve_placeholders() {
    let temp = TempDir::new().expect("tempdir");
    let asset = simple_asset(&temp);
    let catalog = write_file(
        temp.path(),
        "bad-placeholders.po",
        concat!(
            "msgctxt \"availability_reason:trust_too_low\"\n",
            "msgid \"{subject} does not trust {target} enough.\"\n",
            "msgstr \"{actor} does not trust enough.\"\n",
        ),
    );
    let fixture = write_file(
        temp.path(),
        "bad-placeholders.toml",
        &format!(
            "[dialogue]\nlocale = \"fr-FR\"\n\n[dialogue.catalogs]\n\"fr-FR\" = [\"{}\"]\n",
            catalog.display()
        ),
    );

    let output = run_fixture(&asset, &fixture);
    output.assert_failure();
    output.assert_stderr_contains("translation placeholders must match msgid");
    output.assert_stderr_contains("missing {subject}, {target}");
    output.assert_stderr_contains("extra {actor}");
}

#[test]
fn empty_catalog_translations_may_omit_placeholders_for_fallback() {
    let temp = TempDir::new().expect("tempdir");
    let asset = simple_asset(&temp);
    let catalog = write_file(
        temp.path(),
        "empty-placeholders.po",
        concat!(
            "msgctxt \"availability_reason:trust_too_low\"\n",
            "msgid \"{subject} does not trust {target} enough.\"\n",
            "msgstr \"\"\n",
        ),
    );
    let fixture = write_file(
        temp.path(),
        "empty-placeholders.toml",
        &format!(
            "[dialogue]\nlocale = \"fr-FR\"\n\n[dialogue.catalogs]\n\"fr-FR\" = [\"{}\"]\n",
            catalog.display()
        ),
    );

    let output = run_fixture(&asset, &fixture);
    output.assert_success();
}
