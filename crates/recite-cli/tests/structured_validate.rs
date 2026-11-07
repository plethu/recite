#![cfg(test)]

use serde_json::Value;
use tempfile::TempDir;

mod support;
use support::*;

fn records(output: &std::process::Output) -> Vec<Value> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).expect("one JSON record per line"))
        .collect()
}

#[test]
fn structured_validate_honours_schema_and_only_with_the_same_records_as_human_mode() {
    let temp = TempDir::new().expect("tempdir");
    let schema = write_file(
        temp.path(),
        "schema.json",
        r#"{"schema_version":1,"metadata":{"mood":{"targets":["line"],"type":"string","repeatable":false}}}"#,
    );
    // One metadata problem and one identifier problem: `--only metadata` keeps only the first.
    let source = write_recite(
        temp.path(),
        "mixed.recite",
        ":: start default\n> unknown=flat @ffee6p8\n  Hello.\n>\n  Missing id.\n-> END\n",
    );

    let output = run(recite()
        .args([
            "validate",
            "--only",
            "metadata",
            "--output-format",
            "structured",
        ])
        .arg("--schema")
        .arg(&schema)
        .arg(&source));
    output
        .assert_failure()
        .assert_exit_code(1)
        .assert_stderr("");

    let records = records(&output);
    assert_eq!(records[0]["command"], "validate");
    assert_eq!(records[1]["status"], "content_diagnostics");
    let codes = records[1]["data"]["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .iter()
        .map(|diagnostic| diagnostic["code"].as_str().expect("code"))
        .collect::<Vec<_>>();
    assert_eq!(codes, ["RECITE_VALIDATE026"]);
}
