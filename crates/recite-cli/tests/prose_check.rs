#![cfg(test)]

use std::path::{Path, PathBuf};

use serde_json::Value;
use tempfile::TempDir;

mod support;
use support::*;

const MANIFEST: &str = "format_version = 1\n\n[prose]\nlanguage = \"en-GB\"\ndictionaries = [\"words.txt\"]\nmetadata_keys = [\"caption\"]\n";

fn fixture(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    std::fs::read_to_string(path).expect("fixture")
}

/// A project holding the shared prose fixture and its dictionary.
fn project(temp: &Path) -> PathBuf {
    write_project_manifest(temp, MANIFEST);
    write_file(
        temp,
        "words.txt",
        &fixture("fixtures/recite/valid/prose/words.txt"),
    );
    std::fs::create_dir_all(temp.join("dialogue")).expect("dialogue directory");
    write_file(
        &temp.join("dialogue"),
        "harbour.recite",
        &fixture("fixtures/recite/valid/prose/harbour.recite"),
    )
}

fn records(output: &std::process::Output) -> Vec<Value> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).expect("one JSON record per line"))
        .collect()
}

#[test]
fn prose_check_reports_warnings_and_fails_only_this_check() {
    let temp = TempDir::new().expect("tempdir");
    let source = project(temp.path());
    let output = run(recite().args(["prose", "check"]).arg(&source));
    output.assert_failure().assert_exit_code(1);
    output.assert_stderr_contains("warning[RECITE_PROSE001]: possible spelling mistake 'recieved'");
    output.assert_stderr_contains("warning[RECITE_PROSE002]: possible grammar problem in 'an'");
    output.assert_stderr_contains("4 warnings in 1 file");
    output.assert_stderr_not_contains("Vexmoor");
    output.assert_stderr_not_contains("error[");

    // Prose findings never affect validation or compilation.
    run(recite().arg("validate").arg(&source)).assert_success();
}

#[test]
fn structured_prose_check_returns_records_and_byte_exact_fixes() {
    let temp = TempDir::new().expect("tempdir");
    let source_path = project(temp.path());
    let source = std::fs::read_to_string(&source_path).expect("source");
    let output = run(recite()
        .args(["prose", "check", "--output-format", "structured"])
        .args(["--invocation-id", "prose-1"])
        .arg(&source_path));
    output
        .assert_failure()
        .assert_exit_code(1)
        .assert_stderr("");
    let records = records(&output);
    assert_eq!(records[0]["command"], "prose check");
    assert_eq!(records[1]["status"], "content_diagnostics");
    assert_eq!(records[1]["invocation_id"], "prose-1");
    let diagnostics = records[1]["data"]["diagnostics"]
        .as_array()
        .expect("diagnostics");
    let codes = diagnostics
        .iter()
        .map(|diagnostic| diagnostic["code"].as_str().expect("code"))
        .collect::<Vec<_>>();
    assert_eq!(
        codes,
        [
            "RECITE_PROSE002",
            "RECITE_PROSE001",
            "RECITE_PROSE002",
            "RECITE_PROSE001"
        ]
    );
    for fix in records[1]["data"]["fixes"].as_array().expect("fixes") {
        let start = fix["start_byte"].as_u64().expect("start") as usize;
        let end = fix["end_byte"].as_u64().expect("end") as usize;
        let index = fix["diagnostic"].as_u64().expect("index") as usize;
        let flagged = diagnostics[index]["presentation"]["arguments"]["text"]["value"]
            .as_str()
            .expect("flagged text");
        assert_eq!(&source[start..end], flagged);
    }
}

#[test]
fn prose_check_without_a_project_uses_defaults_and_passes_clean_prose() {
    let temp = TempDir::new().expect("tempdir");
    let clean = write_recite(
        temp.path(),
        "clean.recite",
        ":: start default\n> speaker=rhea @hello\n  Hello there.\n-> END\n",
    );
    run(recite().args(["prose", "check"]).arg(&clean))
        .assert_success()
        .assert_stderr_contains("no diagnostics");
}

#[test]
fn unsupported_prose_language_is_a_structured_setup_error() {
    let temp = TempDir::new().expect("tempdir");
    write_project_manifest(
        temp.path(),
        "format_version = 1\n\n[prose]\nlanguage = \"tlh\"\n",
    );
    let source = write_recite(
        temp.path(),
        "start.recite",
        ":: start default\n> @hello\n  Hello.\n",
    );
    let output = run(recite().args(["prose", "check"]).arg(&source));
    output.assert_failure().assert_exit_code(1);
    output.assert_stderr_contains("error[RECITE_PROSE101]: unsupported prose language: tlh");
}
