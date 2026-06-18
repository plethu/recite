#![cfg(test)]

use tempfile::TempDir;

mod support;
use support::*;

#[test]
fn explain_prints_known_diagnostic_guidance() {
    let output = run(recite().arg("explain").arg("RECITE_PARSE001"));

    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("Code: RECITE_PARSE001");
    output.assert_stdout_contains("Category: parse");
    output.assert_stdout_contains("Meaning:");
    output.assert_stdout_contains("Common causes:");
    output.assert_stdout_contains("How to fix:");
    output.assert_stdout_contains("reported span");
}

#[test]
fn explain_rejects_malformed_diagnostic_code_with_suggestion() {
    let output = run(recite().arg("explain").arg("recite_parse001"));

    output
        .assert_failure()
        .assert_exit_code(1)
        .assert_stdout("");
    output.assert_stderr_contains("error: malformed diagnostic code `recite_parse001`");
    output.assert_stderr_contains("expected an uppercase namespaced code such as RECITE_PARSE001");
    output.assert_stderr_contains("did you mean `RECITE_PARSE001`?");
}

#[test]
fn explain_rejects_unknown_diagnostic_code() {
    let output = run(recite().arg("explain").arg("RECITE_PARSE999"));

    output
        .assert_failure()
        .assert_exit_code(1)
        .assert_stdout("");
    output.assert_stderr_contains("error: unknown diagnostic code `RECITE_PARSE999`");
}

#[test]
fn explain_suggests_close_known_diagnostic_code() {
    let output = run(recite().arg("explain").arg("RECITE_PARSE01"));

    output
        .assert_failure()
        .assert_exit_code(1)
        .assert_stdout("");
    output.assert_stderr_contains("error: unknown diagnostic code `RECITE_PARSE01`");
    output.assert_stderr_contains("did you mean `RECITE_PARSE001`?");
}

#[test]
fn explain_with_bad_ui_locale_falls_back_to_default() {
    let temp = TempDir::new().expect("tempdir");
    let bad_config = write_file(
        temp.path(),
        "config.toml",
        "[ui]\nlocale = \"not a locale\"\n",
    );

    let output = run(recite()
        .arg("explain")
        .arg("RECITE_PARSE001")
        .env("RECITE_CONFIG", &bad_config));

    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("Code: RECITE_PARSE001");
    output.assert_stdout_contains("Common causes:");
}
