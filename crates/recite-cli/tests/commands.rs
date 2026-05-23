use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

fn recite() -> Command {
    Command::new(env!("CARGO_BIN_EXE_recite"))
}

fn run(command: &mut Command) -> Output {
    command.output().expect("command runs")
}

#[test]
fn validate_reports_diagnostics_and_success() {
    let temp = TempDir::new().expect("tempdir");
    let valid = write_recite(
        temp.path(),
        "valid.recite",
        ":: start default\n> intro\n  Hello.\n-> END\n",
    );
    recite()
        .arg("validate")
        .arg(&valid)
        .assert_success()
        .assert_stderr("");

    let invalid = write_recite(
        temp.path(),
        "invalid.recite",
        ":: start default\n>\n  Missing id.\n",
    );
    let output = run(recite().arg("validate").arg(&invalid));
    output.assert_failure();
    output.assert_stderr_contains("error RECITE_ID001");
    output.assert_stderr_contains("invalid.recite:2:1");
}

#[test]
fn compile_writes_messagepack_only_after_clean_validation() {
    let temp = TempDir::new().expect("tempdir");
    let valid = write_recite(
        temp.path(),
        "valid.recite",
        ":: start default\n> intro\n  Hello.\n-> END\n",
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
    output.assert_failure();
    output.assert_stderr_contains("RECITE_ID001");
    assert_eq!(
        fs::read(&output_path).expect("stale output remains"),
        stale,
        "failed compile must not overwrite existing output"
    );
}

#[test]
fn compile_refuses_to_overwrite_input_source() {
    let temp = TempDir::new().expect("tempdir");
    let source_text = ":: start default\n> intro\n  Hello.\n-> END\n";
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

#[test]
fn extract_emits_pot_to_stdout_or_output_after_validation() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        ":: start default\n> intro\n  Hello.\n  ? ask\n    Ask?\n    -> END\n",
    );

    let output = run(recite().arg("extract").arg(&source));
    output.assert_success();
    output.assert_stdout_contains("msgctxt \"intro\"");
    output.assert_stdout_contains("msgid \"Hello.\"");
    output.assert_stdout_contains("msgctxt \"ask\"");

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
fn extract_refuses_to_overwrite_input_source() {
    let temp = TempDir::new().expect("tempdir");
    let source_text = ":: start default\n> intro\n  Hello.\n-> END\n";
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

#[test]
fn check_ids_reports_only_id_diagnostics() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "ids.recite",
        ":: start\n>\n  Missing id and missing default.\n> repeated\n  First.\n> repeated\n  Second.\n",
    );

    let output = run(recite().arg("check-ids").arg(&source));
    output.assert_failure();
    output.assert_stderr_contains("RECITE_ID001");
    output.assert_stderr_contains("RECITE_ID003");
    output.assert_stderr_not_contains("RECITE_VALIDATE005");

    let unrelated = write_recite(
        temp.path(),
        "unrelated.recite",
        ":: start\n> intro\n  IDs are present, but no default block.\n",
    );
    recite()
        .arg("check-ids")
        .arg(unrelated)
        .assert_success()
        .assert_stderr("");
}

#[test]
fn check_markup_uses_schema_when_supplied_and_skips_schema_policy_without_one() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "markup.recite",
        ":: start default\n> intro\n  [ghost]Hello[/ghost]\n-> END\n",
    );

    recite()
        .arg("check-markup")
        .arg(&source)
        .assert_success()
        .assert_stderr("");

    let malformed = write_recite(
        temp.path(),
        "malformed.recite",
        ":: start default\n> intro\n  Hello.\n    Mixed indent.\n",
    );
    let output = run(recite().arg("check-markup").arg(malformed));
    output.assert_failure();
    output.assert_stderr_contains("RECITE_PARSE007");

    let schema = write_file(
        temp.path(),
        "schema.json",
        r#"{"schema_version":1,"markup":{"em":{"requires_closing":true,"translatable":true,"allows_nesting":true}}}"#,
    );
    let output = run(recite()
        .arg("check-markup")
        .arg("--schema")
        .arg(schema)
        .arg(source));
    output.assert_failure();
    output.assert_stderr_contains("RECITE_VALIDATE022");
}

#[test]
fn check_metadata_requires_schema_and_reports_schema_validation() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "metadata.recite",
        ":: start default\n> intro mood=angry\n  Hello.\n-> END\n",
    );
    let schema = write_file(
        temp.path(),
        "schema.json",
        r#"{"schema_version":1,"metadata":{"mood":{"targets":["line"],"type":"string","repeatable":false}}}"#,
    );

    recite()
        .arg("check-metadata")
        .arg("--schema")
        .arg(&schema)
        .arg(&source)
        .assert_success();

    let invalid_metadata = write_recite(
        temp.path(),
        "invalid-metadata.recite",
        ":: start default\n> intro unknown=flat\n  Hello.\n-> END\n",
    );
    let output = run(recite()
        .arg("check-metadata")
        .arg("--schema")
        .arg(&schema)
        .arg(invalid_metadata));
    output.assert_failure();
    output.assert_stderr_contains("RECITE_VALIDATE026");

    let bad_schema = write_file(
        temp.path(),
        "bad-schema.json",
        r#"{"schema_version":"one"}"#,
    );
    let output = run(recite()
        .arg("check-metadata")
        .arg("--schema")
        .arg(bad_schema)
        .arg(source));
    output.assert_failure();
    output.assert_stderr_contains("RECITE_SCHEMA001");
}

fn write_recite(root: &Path, name: &str, source: &str) -> PathBuf {
    write_file(root, name, source)
}

fn write_file(root: &Path, name: &str, source: &str) -> PathBuf {
    let path = root.join(name);
    fs::write(&path, source).expect("write test file");
    path
}

trait CommandExt {
    fn assert_success(&mut self) -> Output;
}

impl CommandExt for Command {
    fn assert_success(&mut self) -> Output {
        let output = run(self);
        output.assert_success();
        output
    }
}

trait OutputExt {
    fn assert_success(&self) -> &Self;
    fn assert_failure(&self) -> &Self;
    fn assert_stdout(&self, expected: &str) -> &Self;
    fn assert_stderr(&self, expected: &str) -> &Self;
    fn assert_stdout_contains(&self, expected: &str);
    fn assert_stderr_contains(&self, expected: &str);
    fn assert_stderr_not_contains(&self, unexpected: &str);
}

impl OutputExt for Output {
    fn assert_success(&self) -> &Self {
        assert!(
            self.status.success(),
            "expected success\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            self.status,
            stdout(self),
            stderr(self)
        );
        self
    }

    fn assert_failure(&self) -> &Self {
        assert!(
            !self.status.success(),
            "expected failure\nstdout:\n{}\nstderr:\n{}",
            stdout(self),
            stderr(self)
        );
        self
    }

    fn assert_stdout(&self, expected: &str) -> &Self {
        assert_eq!(stdout(self), expected);
        self
    }

    fn assert_stderr(&self, expected: &str) -> &Self {
        assert_eq!(stderr(self), expected);
        self
    }

    fn assert_stdout_contains(&self, expected: &str) {
        assert!(
            stdout(self).contains(expected),
            "stdout did not contain {expected:?}\nstdout:\n{}",
            stdout(self)
        );
    }

    fn assert_stderr_contains(&self, expected: &str) {
        assert!(
            stderr(self).contains(expected),
            "stderr did not contain {expected:?}\nstderr:\n{}",
            stderr(self)
        );
    }

    fn assert_stderr_not_contains(&self, unexpected: &str) {
        assert!(
            !stderr(self).contains(unexpected),
            "stderr unexpectedly contained {unexpected:?}\nstderr:\n{}",
            stderr(self)
        );
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
