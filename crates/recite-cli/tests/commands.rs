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
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("error RECITE_ID001");
    output.assert_stderr_contains("invalid.recite:2:1");
}

#[test]
fn operational_failures_keep_error_prefix() {
    let temp = TempDir::new().expect("tempdir");
    let output = run(recite().arg("validate").arg(temp.path()));

    output.assert_failure();
    output.assert_exit_code(1);
    output.assert_stderr("error: no .recite inputs found\n");
}

#[test]
fn help_covers_issue_25_commands_and_options() {
    let output = run(recite().arg("--help"));
    output.assert_success().assert_stderr("");
    for command in [
        "validate",
        "compile",
        "extract",
        "check-ids",
        "check-markup",
        "check-metadata",
        "validate-project",
        "check-fresh",
    ] {
        output.assert_stdout_contains(command);
    }

    let compile = run(recite().arg("compile").arg("--help"));
    compile.assert_success().assert_stderr("");
    compile.assert_stdout_contains("Usage: recite compile [OPTIONS] --output <OUTPUT> <PATHS>...");
    compile.assert_stdout_contains("--schema <SCHEMA>");

    let extract = run(recite().arg("extract").arg("--help"));
    extract.assert_success().assert_stderr("");
    extract.assert_stdout_contains("--output <OUTPUT>");
    extract.assert_stdout_contains("--schema <SCHEMA>");

    let metadata = run(recite().arg("check-metadata").arg("--help"));
    metadata.assert_success().assert_stderr("");
    metadata.assert_stdout_contains("--schema <SCHEMA>");

    let project = run(recite().arg("validate-project").arg("--help"));
    project.assert_success().assert_stderr("");
    project.assert_stdout_contains("recite.project.toml");

    let fresh = run(recite().arg("check-fresh").arg("--help"));
    fresh.assert_success().assert_stderr("");
    fresh.assert_stdout_contains("compiled assets are fresh");
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
        ":: start default\n> intro\n  Hello.\n-> END\n",
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
        ":: start default\n> intro\n  Hello.\n-> END\n",
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
    assert_diagnostic_failure(&output);
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
    assert_diagnostic_failure(&output);
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
    assert_diagnostic_failure(&output);
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
    assert_diagnostic_failure(&output);
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
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_SCHEMA001");
}

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
        ":: start default speaker=hazel\n> intro\n  Shadow source.\n-> END\n",
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
        ":: start default speaker=hazel\n> intro\n  Changed.\n-> END\n",
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

fn assert_diagnostic_failure(output: &Output) {
    output.assert_failure();
    output.assert_exit_code(1);
    output.assert_stderr_not_contains("error: diagnostics reported");
}

fn write_recite(root: &Path, name: &str, source: &str) -> PathBuf {
    write_file(root, name, source)
}

fn write_project_manifest(root: &Path, source: &str) -> PathBuf {
    write_file(root, "recite.project.toml", source)
}

fn write_file(root: &Path, name: &str, source: &str) -> PathBuf {
    let path = root.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create test parent directory");
    }
    fs::write(&path, source).expect("write test file");
    path
}

fn compile_project_asset(
    root: &Path,
    source: &Path,
    asset_name: &str,
    schema: Option<&Path>,
) -> PathBuf {
    let output = root.join(asset_name);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).expect("create compiled asset parent directory");
    }
    let mut command = recite();
    command.arg("compile").arg("--output").arg(&output);
    if let Some(schema) = schema {
        command.arg("--schema").arg(schema);
    }
    let source = source.strip_prefix(root).unwrap_or(source);
    command.current_dir(root).arg(source).assert_success();
    output
}

fn corrupt_compiler_compatibility(asset: &Path) {
    let mut bytes = fs::read(asset).expect("compiled asset bytes");
    assert_eq!(bytes[0], 0x9f, "compiled dialogue is a 15-field array");
    assert_eq!(bytes[1], 0x98, "asset header is an 8-field array");
    assert_eq!(bytes[2], 0, "format version starts at v0");
    bytes[3] = 1;
    fs::write(asset, bytes).expect("corrupt compiler compatibility");
}

fn project_source() -> &'static str {
    ":: start default speaker=hazel\n> intro\n  Hello.\n-> END\n"
}

fn schema_manifest<const N: usize>(speakers: [&str; N]) -> String {
    let speakers = speakers
        .into_iter()
        .map(|speaker| format!(r#""{speaker}":{{}}"#))
        .collect::<Vec<_>>()
        .join(",");
    format!(r#"{{"schema_version":1,"speakers":{{{speakers}}}}}"#)
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
    fn assert_exit_code(&self, expected: i32) -> &Self;
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

    fn assert_exit_code(&self, expected: i32) -> &Self {
        assert_eq!(
            self.status.code(),
            Some(expected),
            "unexpected exit code\nstdout:\n{}\nstderr:\n{}",
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
