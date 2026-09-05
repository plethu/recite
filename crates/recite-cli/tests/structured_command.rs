#![cfg(test)]

use std::fs;
use std::path::PathBuf;

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
fn structured_validate_is_two_records_and_keeps_diagnostics_locale_neutral() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "invalid.recite",
        ":: start default\n>\n  Missing id.\n",
    );

    let output = run(recite()
        .arg("validate")
        .arg("--output-format")
        .arg("structured")
        .arg("--invocation-id")
        .arg("validate-1")
        .arg(&source));
    output
        .assert_failure()
        .assert_exit_code(1)
        .assert_stderr("");

    let records = records(&output);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["version"], 1);
    assert_eq!(records[0]["sequence"], 0);
    assert_eq!(records[0]["event"], "command.started");
    assert_eq!(records[0]["invocation_id"], "validate-1");
    assert_eq!(records[1]["sequence"], 1);
    assert_eq!(records[1]["event"], "command.result");
    assert_eq!(records[1]["invocation_id"], "validate-1");
    assert_eq!(records[1]["status"], "content_diagnostics");
    assert_eq!(records[1]["exit_code"], 1);
    assert_eq!(records[1]["data"]["diagnostics"][0]["code"], "RECITE_ID001");
    assert!(
        records[1]["data"]["diagnostics"][0]
            .get("compatibility_message")
            .is_some()
    );
}

#[test]
fn structured_compile_and_extract_project_machine_artifacts_or_typed_entries() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> intro@11111111111111111111\n  Hello.\n-> END\n",
    );
    let asset = temp.path().join("dialogue.recitec");

    let compile = run(recite()
        .arg("compile")
        .arg("--output-format")
        .arg("structured")
        .arg("--output")
        .arg(&asset)
        .arg(&source));
    compile.assert_success().assert_stderr("");
    let compile_records = records(&compile);
    assert_eq!(compile_records.len(), 2);
    assert_eq!(
        compile_records[1]["data"]["artifact"]["path"]["encoding"],
        "utf8"
    );
    assert_eq!(
        compile_records[1]["data"]["artifact"]["path"]["value"],
        asset.to_string_lossy().as_ref()
    );
    assert_eq!(
        compile_records[1]["data"]["artifact"]["size_bytes"],
        fs::metadata(&asset).expect("artifact metadata").len()
    );

    let extract = run(recite()
        .arg("extract")
        .arg("--output-format")
        .arg("structured")
        .arg(&source));
    extract.assert_success().assert_stderr("");
    let extract_records = records(&extract);
    assert_eq!(extract_records.len(), 2);
    assert_eq!(
        extract_records[1]["data"]["entries"][0]["context"],
        "11111111111111111111"
    );
    assert_eq!(
        extract_records[1]["data"]["entries"][0]["source_text"],
        "Hello."
    );
    assert!(extract_records[1]["data"].get("pot").is_none());

    let pot = temp.path().join("dialogue.pot");
    let extract_file = run(recite()
        .arg("extract")
        .arg("--output-format")
        .arg("structured")
        .arg("--output")
        .arg(&pot)
        .arg(&source));
    extract_file.assert_success().assert_stderr("");
    let extract_file_records = records(&extract_file);
    assert!(extract_file_records[1]["data"].get("entries").is_none());
    assert_eq!(
        extract_file_records[1]["data"]["artifact"]["size_bytes"],
        fs::metadata(pot).expect("pot metadata").len()
    );
}

#[test]
fn structured_compile_and_extract_invalid_source_are_result_diagnostics() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "invalid.recite",
        ":: start default\n>\n  Missing id.\n",
    );
    let asset = temp.path().join("dialogue.recitec");
    let compile = run(recite()
        .arg("compile")
        .arg("--output-format")
        .arg("structured")
        .arg("--output")
        .arg(&asset)
        .arg(&source));
    compile
        .assert_failure()
        .assert_exit_code(1)
        .assert_stderr("");
    let compile_records = records(&compile);
    assert_eq!(compile_records.len(), 2);
    assert_eq!(compile_records[1]["event"], "command.result");
    assert_eq!(
        compile_records[1]["data"]["diagnostics"][0]["code"],
        "RECITE_ID001"
    );

    let extract = run(recite()
        .arg("extract")
        .arg("--output-format")
        .arg("structured")
        .arg(&source));
    extract
        .assert_failure()
        .assert_exit_code(1)
        .assert_stderr("");
    let extract_records = records(&extract);
    assert_eq!(extract_records.len(), 2);
    assert_eq!(extract_records[1]["event"], "command.result");
    assert_eq!(
        extract_records[1]["data"]["diagnostics"][0]["code"],
        "RECITE_ID001"
    );
}

#[test]
fn structured_run_and_trace_share_deterministic_trace_data_without_run_text() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> intro@11111111111111111111\n  Hello.\n-> END\n",
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let fixture = write_file(temp.path(), "fixture.toml", "");

    let run_output = run(recite()
        .arg("run")
        .arg("--output-format")
        .arg("structured")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    let trace_output = run(recite()
        .arg("trace")
        .arg("--output-format")
        .arg("structured")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    run_output.assert_success().assert_stderr("");
    trace_output.assert_success().assert_stderr("");

    let run_records = records(&run_output);
    let trace_records = records(&trace_output);
    assert_eq!(run_records.len(), 2);
    assert_eq!(trace_records.len(), 2);
    assert_eq!(
        run_records[1]["data"]["trace"],
        trace_records[1]["data"]["trace"]
    );
    assert_eq!(run_records[1]["data"]["trace"]["events"][0]["type"], "line");
    assert_eq!(
        run_records[1]["data"]["trace"]["events"][0]["line"]["text"],
        "Hello."
    );
    assert!(String::from_utf8_lossy(&run_output.stdout).contains("command.result"));
    assert!(
        !String::from_utf8_lossy(&run_output.stdout).contains("line 11111111111111111111: Hello.")
    );
}

#[test]
fn structured_fatal_failure_has_typed_category_operation_and_exact_path() {
    let temp = TempDir::new().expect("tempdir");
    let missing = temp.path().join("missing.recite");
    let output = run(recite()
        .arg("validate")
        .arg("--output-format")
        .arg("structured")
        .arg(&missing));
    output
        .assert_failure()
        .assert_exit_code(1)
        .assert_stderr("");

    let records = records(&output);
    assert_eq!(records.len(), 2);
    assert_eq!(records[1]["event"], "command.error");
    assert_eq!(records[1]["error"]["category"], "input");
    assert_eq!(records[1]["error"]["operation"], "resolve_path");
    assert_eq!(records[1]["error"]["path"]["encoding"], "utf8");
    assert_eq!(
        records[1]["error"]["path"]["value"],
        missing.to_string_lossy().as_ref()
    );
}

#[test]
fn structured_rejects_fixture_choices_with_typed_details() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "choice.recite",
        concat!(
            ":: start default\n",
            "> intro@11111111111111111111\n",
            "  Choose.\n",
            "  ? left@22222222222222222222\n",
            "    Left.\n",
            "    -> END\n",
            "  ? right@33333333333333333333\n",
            "    Right.\n",
            "    -> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "choice.recitec", None);
    let fixture = write_file(
        temp.path(),
        "invalid-choice.toml",
        "[choices]\n11111111111111111111 = \"not-a-choice\"\n",
    );

    let output = run(recite()
        .arg("run")
        .arg("--output-format")
        .arg("structured")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    output
        .assert_failure()
        .assert_exit_code(1)
        .assert_stderr("");
    let records = records(&output);
    assert_eq!(records.len(), 2);
    assert_eq!(records[1]["event"], "command.error");
    assert_eq!(records[1]["error"]["category"], "fixture");
    assert_eq!(records[1]["error"]["code"], "fixture_choice_not_in_prompt");
    assert_eq!(records[1]["error"]["operation"], "select_fixture_choice");
    assert_eq!(
        records[1]["error"]["path"]["value"],
        fixture.to_string_lossy().as_ref()
    );
    assert_eq!(records[1]["error"]["details"]["type"], "fixture_choice");
    assert_eq!(records[1]["error"]["details"]["choice"], "not-a-choice");
}

#[test]
fn structured_rejects_malformed_asset_with_exact_path() {
    let temp = TempDir::new().expect("tempdir");
    let asset = write_file(temp.path(), "broken.recitec", "not a messagepack asset");
    let fixture = write_file(temp.path(), "fixture.toml", "");

    let output = run(recite()
        .arg("trace")
        .arg("--output-format")
        .arg("structured")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    output
        .assert_failure()
        .assert_exit_code(1)
        .assert_stderr("");
    let records = records(&output);
    assert_eq!(records[1]["event"], "command.error");
    assert_eq!(records[1]["error"]["category"], "asset");
    assert_eq!(records[1]["error"]["code"], "decode_asset");
    assert_eq!(records[1]["error"]["path"]["encoding"], "utf8");
    assert_eq!(
        records[1]["error"]["path"]["value"],
        asset.to_string_lossy().as_ref()
    );
}

#[test]
fn structured_rejects_output_input_alias_with_both_paths() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> intro@11111111111111111111\n  Hello.\n-> END\n",
    );

    let output = run(recite()
        .arg("compile")
        .arg("--output-format")
        .arg("structured")
        .arg("--output")
        .arg(&source)
        .arg(&source));
    output
        .assert_failure()
        .assert_exit_code(1)
        .assert_stderr("");
    let records = records(&output);
    assert_eq!(records[1]["event"], "command.error");
    assert_eq!(records[1]["error"]["category"], "input");
    assert_eq!(records[1]["error"]["code"], "output_overwrites_input");
    assert_eq!(records[1]["error"]["operation"], "write_output");
    assert_eq!(
        records[1]["error"]["path"]["value"],
        source.to_string_lossy().as_ref()
    );
    assert_eq!(
        records[1]["error"]["related_path"]["value"],
        source.to_string_lossy().as_ref()
    );
}

#[test]
fn structured_schema_diagnostics_are_typed_content_results() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> intro@11111111111111111111\n  Hello.\n-> END\n",
    );
    let schema = write_file(temp.path(), "schema.json", "{}");
    let asset = temp.path().join("dialogue.recitec");

    let output = run(recite()
        .arg("compile")
        .arg("--output-format")
        .arg("structured")
        .arg("--schema")
        .arg(&schema)
        .arg("--output")
        .arg(&asset)
        .arg(&source));
    output
        .assert_failure()
        .assert_exit_code(1)
        .assert_stderr("");
    let records = records(&output);
    assert_eq!(records[1]["event"], "command.result");
    assert_eq!(records[1]["status"], "content_diagnostics");
    assert_eq!(
        records[1]["data"]["diagnostics"][0]["code"],
        "RECITE_SCHEMA001"
    );
}

#[test]
fn structured_output_is_invariant_under_user_ui_configuration() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> intro@11111111111111111111\n  Hello.\n-> END\n",
    );
    let configured = write_file(temp.path(), "configured.toml", "[ui]\nlocale = \"en-GB\"\n");
    let malformed = write_file(temp.path(), "malformed.toml", "[ui\nlocale =");

    let baseline = run(recite()
        .arg("validate")
        .arg("--output-format")
        .arg("structured")
        .arg(&source));
    baseline.assert_success().assert_stderr("");
    for config in [configured, malformed] {
        let output = run(recite()
            .arg("validate")
            .arg("--output-format")
            .arg("structured")
            .arg(&source)
            .env("RECITE_CONFIG", config));
        output.assert_success().assert_stderr("");
        assert_eq!(output.stdout, baseline.stdout);
        assert_eq!(output.stderr, baseline.stderr);
    }
}

#[test]
fn human_run_keeps_the_existing_output_bytes() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> intro@11111111111111111111\n  Hello.\n-> END\n",
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let fixture = write_file(temp.path(), "fixture.toml", "");
    let output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    output
        .assert_success()
        .assert_stdout("line 11111111111111111111: Hello.\nend\n")
        .assert_stderr("");
}

#[test]
fn structured_flags_have_useful_help_text() {
    let output = run(recite().arg("validate").arg("--help"));
    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("--output-format <OUTPUT_FORMAT>");
    output.assert_stdout_contains("version-1 newline-delimited structured output");
    output.assert_stdout_contains("--invocation-id <INVOCATION_ID>");
    output.assert_stdout_contains("Caller-owned identifier");
}

#[cfg(unix)]
#[test]
fn structured_error_preserves_non_utf8_path_projection() {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let temp = TempDir::new().expect("tempdir");
    let name = std::ffi::OsString::from_vec(b"missing-\xff.recite".to_vec());
    let missing = PathBuf::from(temp.path()).join(name);
    let output = run(recite()
        .arg("validate")
        .arg("--output-format")
        .arg("structured")
        .arg(&missing));
    output
        .assert_failure()
        .assert_exit_code(1)
        .assert_stderr("");
    let records = records(&output);
    assert_eq!(records[1]["event"], "command.error");
    assert_eq!(records[1]["error"]["path"]["encoding"], "unix_bytes");
    let expected = missing
        .as_os_str()
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(records[1]["error"]["path"]["value"], expected);
}
