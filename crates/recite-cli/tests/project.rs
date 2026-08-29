#![cfg(test)]

use std::fs;

use tempfile::TempDir;

mod support;
use support::*;

#[test]
fn validate_project_accepts_fresh_manifest_and_asset() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(temp.path(), "dialogue.recite", project_source());
    compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    write_project_manifest(
        temp.path(),
        r#"[project]
content_set = "base"
version = "0.1.0"

[[scenes]]
id = "scene.start"
presentation = "portrait_dialogue"
asset = "dialogue.recitec"
block = "start"
participants = ["hazel"]
"#,
    );

    recite()
        .arg("validate-project")
        .arg(temp.path())
        .assert_success()
        .assert_stderr("");
    recite()
        .arg("check-fresh")
        .arg(temp.path())
        .assert_success()
        .assert_stderr("");
}

#[test]
fn check_schema_producer_freshness_reports_structured_stale_content() {
    let temp = TempDir::new().expect("tempdir");
    let expected = write_file(
        temp.path(),
        "expected.json",
        r#"{"schema_version":1,"producer":{"kind":"adapter","id":"items"},"content_fingerprint":{"algorithm":"blake3","value":"0000000000000000000000000000000000000000000000000000000000000000"}}"#,
    );
    let actual = write_file(
        temp.path(),
        "actual.json",
        r#"{"schema_version":1,"producer":{"kind":"adapter","id":"items"},"content_fingerprint":{"algorithm":"blake3","value":"1111111111111111111111111111111111111111111111111111111111111111"}}"#,
    );
    let output = run(recite()
        .current_dir(temp.path())
        .arg("check-schema-producer-freshness")
        .arg("--expected")
        .arg(&expected)
        .arg("--actual")
        .arg(&actual));
    output.assert_failure().assert_exit_code(1);
    output.assert_stdout_contains("\"status\":\"mismatch\"");
    output.assert_stdout_contains("\"expected\"");
}

#[test]
fn check_schema_producer_freshness_reports_duplicate_fingerprints_as_invalid_json() {
    let temp = TempDir::new().expect("tempdir");
    let expected = write_file(
        temp.path(),
        "expected.json",
        r#"{
  "schema_version": 1,
  "producer_fingerprints": [
    { "id": "items", "kind": "directory", "algorithm": "blake3", "value": "one" },
    { "id": "items", "kind": "directory", "algorithm": "blake3", "value": "two" }
  ]
}"#,
    );
    let actual = write_file(
        temp.path(),
        "actual.json",
        r#"{
  "schema_version": 1,
  "producer_fingerprints": [
    { "id": "items", "kind": "directory", "algorithm": "blake3", "value": "one" }
  ]
}"#,
    );
    let output = run(recite()
        .current_dir(temp.path())
        .arg("check-schema-producer-freshness")
        .arg("--expected")
        .arg(&expected)
        .arg("--actual")
        .arg(&actual));
    output.assert_failure().assert_exit_code(1);
    output.assert_stdout_contains("\"status\":\"invalid\"");
    output.assert_stdout_contains("expected_duplicates");
    output.assert_stderr("");
}

#[test]
fn check_fresh_uses_project_relative_embedded_source_paths() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "Dialogue/Source/dialogue.recite",
        project_source(),
    );
    compile_project_asset(
        temp.path(),
        &source,
        "Dialogue/Compiled/dialogue.recitec",
        None,
    );
    write_recite(
        temp.path(),
        "Source/dialogue.recite",
        ":: start default speaker=hazel\n> intro@11111111111111111111\n  Shadow source.\n-> END\n",
    );
    write_project_manifest(
        temp.path(),
        r#"[[scenes]]
id = "scene.start"
asset = "Dialogue/Compiled/dialogue.recitec"
block = "start"
participants = ["hazel"]
"#,
    );

    recite()
        .arg("check-fresh")
        .arg(temp.path())
        .assert_success()
        .assert_stderr("");
}

#[test]
fn validate_project_reports_manifest_and_asset_diagnostics() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(temp.path(), "dialogue.recite", project_source());
    compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    write_project_manifest(
        temp.path(),
        r#"[[scenes]]
id = "scene.duplicate"
asset = "dialogue.recitec"
block = "missing_block"

[[scenes]]
id = "scene.duplicate"
asset = "missing.recitec"
block = "start"
participants = ["hazel"]
"#,
    );

    let output = run(recite().arg("validate-project").arg(temp.path()));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_PROJECT002");
    output.assert_stderr_contains("RECITE_PROJECT003");
    output.assert_stderr_contains("RECITE_PROJECT004");
    output.assert_stderr_contains("RECITE_PROJECT005");
}

#[test]
fn validate_project_reports_unknown_fields_and_participants() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(temp.path(), "dialogue.recite", project_source());
    let schema = write_file(temp.path(), "schema.json", &schema_manifest(["hazel"]));
    compile_project_asset(temp.path(), &source, "dialogue.recitec", Some(&schema));
    write_project_manifest(
        temp.path(),
        r#"[project]
schema = "schema.json"

[[scenes]]
id = "scene.start"
asset = "dialogue.recitec"
block = "start"
participants = ["rhea"]
"#,
    );

    let output = run(recite().arg("validate-project").arg(temp.path()));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_PROJECT008");

    write_project_manifest(
        temp.path(),
        r#"unknown = true

[[scenes]]
id = "scene.start"
asset = "dialogue.recitec"
block = "start"
participants = ["hazel"]
"#,
    );

    let output = run(recite().arg("validate-project").arg(temp.path()));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_PROJECT001");
}

#[test]
fn check_fresh_reports_stale_source_schema_and_compiler_compatibility() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(temp.path(), "dialogue.recite", project_source());
    let schema = write_file(temp.path(), "schema.json", &schema_manifest(["hazel"]));
    compile_project_asset(temp.path(), &source, "dialogue.recitec", Some(&schema));
    write_project_manifest(
        temp.path(),
        r#"[project]
schema = "schema.json"

[[scenes]]
id = "scene.start"
asset = "dialogue.recitec"
block = "start"
participants = ["hazel"]
"#,
    );

    fs::write(
        &source,
        ":: start default speaker=hazel\n> intro@11111111111111111111\n  Changed.\n-> END\n",
    )
    .expect("stale source");
    fs::write(&schema, schema_manifest(["hazel", "rhea"])).expect("stale schema");
    let output = run(recite().arg("check-fresh").arg(temp.path()));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_FRESH001");
    output.assert_stderr_contains("RECITE_FRESH002");

    fs::write(&source, project_source()).expect("restore source");
    fs::write(&schema, schema_manifest(["hazel"])).expect("restore schema");
    compile_project_asset(temp.path(), &source, "dialogue.recitec", Some(&schema));
    corrupt_compiler_compatibility(&temp.path().join("dialogue.recitec"));
    let output = run(recite().arg("check-fresh").arg(temp.path()));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_FRESH003");
}

#[test]
fn check_fresh_keeps_producer_metadata_out_of_schema_identity() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> intro@637b1854a7f3ed42f045\n  Hello.\n-> END\n",
    );
    let schema = write_file(
        temp.path(),
        "schema.json",
        include_str!("../../../fixtures/schema/valid/full_manifest.json"),
    );
    compile_project_asset(temp.path(), &source, "dialogue.recitec", Some(&schema));
    write_project_manifest(
        temp.path(),
        r#"[project]
schema = "schema.json"

[[scenes]]
id = "scene.start"
asset = "dialogue.recitec"
block = "start"
participants = ["rhea"]
"#,
    );

    recite()
        .arg("check-fresh")
        .arg(temp.path())
        .assert_success()
        .assert_stderr("");

    let mut changed_schema = include_str!("../../../fixtures/schema/valid/full_manifest.json")
        .replace("dialogue-export-v1", "dialogue-export-v2");
    changed_schema = changed_schema.replace("\"value\": \"6f1d\"", "\"value\": \"changed\"");
    fs::write(&schema, changed_schema).expect("change producer metadata");
    recite()
        .arg("check-fresh")
        .arg(temp.path())
        .assert_success()
        .assert_stderr("");
}

#[test]
fn check_fresh_skips_schema_freshness_when_project_schema_is_invalid() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(temp.path(), "dialogue.recite", project_source());
    let schema = write_file(temp.path(), "schema.json", &schema_manifest(["hazel"]));
    compile_project_asset(temp.path(), &source, "dialogue.recitec", Some(&schema));
    write_project_manifest(
        temp.path(),
        r#"[project]
schema = "schema.json"

[[scenes]]
id = "scene.start"
asset = "dialogue.recitec"
block = "start"
participants = ["hazel"]
"#,
    );

    fs::write(&schema, r#"{"schema_version":"one"}"#).expect("invalid schema");
    let output = run(recite().arg("check-fresh").arg(temp.path()));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_SCHEMA001");
    output.assert_stderr_not_contains("RECITE_FRESH002");
}

#[test]
fn check_fresh_reports_missing_embedded_sources() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(temp.path(), "dialogue.recite", project_source());
    compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    write_project_manifest(
        temp.path(),
        r#"[[scenes]]
id = "scene.start"
asset = "dialogue.recitec"
block = "start"
participants = ["hazel"]
"#,
    );

    fs::remove_file(source).expect("remove compiled source");
    let output = run(recite().arg("check-fresh").arg(temp.path()));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_PROJECT006");
}
