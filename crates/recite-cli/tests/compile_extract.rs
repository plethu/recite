#![cfg(test)]

use std::fs;

use tempfile::TempDir;

mod support;
use support::*;

#[test]
fn compile_writes_messagepack_only_after_clean_validation() {
    let temp = TempDir::new().expect("tempdir");
    let valid = write_recite(
        temp.path(),
        "valid.recite",
        ":: start default\n> @ffee6p8\n  Hello.\n-> END\n",
    );
    let output_path = temp.path().join("dialogue.recitec");

    recite()
        .arg("compile")
        .arg("--output")
        .arg(&output_path)
        .arg(&valid)
        .assert_success();
    let bytes = fs::read(&output_path).expect("compiled asset written");
    assert!(
        !bytes.is_empty(),
        "compiled asset should contain MessagePack"
    );

    let invalid = write_recite(
        temp.path(),
        "invalid.recite",
        ":: start default\n>\n  Missing id.\n",
    );
    let stale = b"do not overwrite";
    fs::write(&output_path, stale).expect("stale output");

    let output = run(recite()
        .arg("compile")
        .arg("--output")
        .arg(&output_path)
        .arg(&invalid));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_ID001");
    assert_eq!(
        fs::read(&output_path).expect("stale output remains"),
        stale,
        "failed compile must not overwrite existing output"
    );
}

#[test]
fn compile_does_not_write_output_when_schema_fails_to_load() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "valid.recite",
        ":: start default\n> @ffee6p8\n  Hello.\n-> END\n",
    );
    let bad_schema = write_file(
        temp.path(),
        "bad-schema.json",
        r#"{"schema_version":"one"}"#,
    );
    let output_path = temp.path().join("dialogue.recitec");
    let stale = b"do not overwrite";
    fs::write(&output_path, stale).expect("stale output");

    let output = run(recite()
        .arg("compile")
        .arg("--output")
        .arg(&output_path)
        .arg("--schema")
        .arg(bad_schema)
        .arg(source));

    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_SCHEMA001");
    assert_eq!(
        fs::read(&output_path).expect("stale output remains"),
        stale,
        "failed schema load must not overwrite existing output"
    );
}

#[test]
fn compile_refuses_to_overwrite_input_source() {
    let temp = TempDir::new().expect("tempdir");
    let source_text = ":: start default\n> @ffee6p8\n  Hello.\n-> END\n";
    let source = write_recite(temp.path(), "source.recite", source_text);

    let output = run(recite()
        .arg("compile")
        .arg("--output")
        .arg(&source)
        .arg(&source));

    output.assert_failure();
    output.assert_stderr_contains("refusing to overwrite input");
    assert_eq!(
        fs::read_to_string(&source).expect("source remains readable"),
        source_text,
        "failed compile must not replace the source file"
    );
}

/// Effect IDs embed the source name, which `compile` takes relative to the directory shared by the
/// output and its inputs. Hosts that rebuild an effect ID must compile where that name is known;
/// the FFI locale-lifetime probe (#194) compiles a copy beside its output for this reason.
#[test]
fn compile_names_effect_sources_relative_to_the_output_and_inputs() {
    let temp = TempDir::new().expect("tempdir");
    let nested = temp.path().join("nested");
    fs::create_dir(&nested).expect("nested dir");
    let source = write_recite(
        &nested,
        "scene.recite",
        ":: start default\n> @ffee6p8\n  Hello.\n! blocking finish()\n-> END\n",
    );

    for (output, expected) in [
        (
            temp.path().join("root.recitec"),
            "effect:nested/scene.recite:4:1",
        ),
        (nested.join("beside.recitec"), "effect:scene.recite:4:1"),
    ] {
        recite()
            .arg("compile")
            .arg("--output")
            .arg(&output)
            .arg(&source)
            .assert_success();
        let bytes = fs::read(&output).expect("compiled asset written");
        assert!(
            bytes
                .windows(expected.len())
                .any(|window| window == expected.as_bytes()),
            "{} should contain {expected}",
            output.display()
        );
    }
}

#[test]
fn extract_emits_pot_to_stdout_or_output_after_validation() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> @ffee6p8\n  Hello.\n  ? @rthh7ab\n    Ask?\n    -> END\n",
    );

    let output = run(recite().arg("extract").arg(&source));
    output.assert_success();
    output.assert_stdout_contains("msgctxt \"ffee6p8\"");
    output.assert_stdout_contains("msgid \"Hello.\"");
    output.assert_stdout_contains("msgctxt \"rthh7ab\"");

    let pot_path = temp.path().join("dialogue.pot");
    recite()
        .arg("extract")
        .arg("--output")
        .arg(&pot_path)
        .arg(&source)
        .assert_success()
        .assert_stdout("");
    assert!(
        fs::read_to_string(pot_path)
            .expect("pot file")
            .contains("msgid \"Ask?\"")
    );
}

#[test]
fn extract_does_not_write_output_on_validation_or_schema_failure() {
    let temp = TempDir::new().expect("tempdir");
    let invalid = write_recite(
        temp.path(),
        "invalid.recite",
        ":: start default\n>\n  Missing id.\n",
    );
    let missing_output = temp.path().join("missing.pot");

    let output = run(recite()
        .arg("extract")
        .arg("--output")
        .arg(&missing_output)
        .arg(&invalid));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_ID001");
    assert!(
        !missing_output.exists(),
        "failed extract must not create a new output file"
    );

    let valid = write_recite(
        temp.path(),
        "valid.recite",
        ":: start default\n> @ffee6p8\n  Hello.\n-> END\n",
    );
    let bad_schema = write_file(
        temp.path(),
        "bad-schema.json",
        r#"{"schema_version":"one"}"#,
    );
    let stale_output = temp.path().join("stale.pot");
    let stale = b"do not overwrite";
    fs::write(&stale_output, stale).expect("stale output");

    let output = run(recite()
        .arg("extract")
        .arg("--output")
        .arg(&stale_output)
        .arg("--schema")
        .arg(bad_schema)
        .arg(valid));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_SCHEMA001");
    assert_eq!(
        fs::read(&stale_output).expect("stale output remains"),
        stale,
        "failed schema load must not overwrite existing output"
    );
}

#[test]
fn extract_refuses_to_overwrite_input_source() {
    let temp = TempDir::new().expect("tempdir");
    let source_text = ":: start default\n> @ffee6p8\n  Hello.\n-> END\n";
    let source = write_recite(temp.path(), "source.recite", source_text);

    let output = run(recite()
        .arg("extract")
        .arg("--output")
        .arg(&source)
        .arg(&source));

    output.assert_failure();
    output.assert_stderr_contains("refusing to overwrite input");
    assert_eq!(
        fs::read_to_string(&source).expect("source remains readable"),
        source_text,
        "failed extract must not replace the source file"
    );
}
