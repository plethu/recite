use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use recite_core::schema::load_schema_manifest_str;
use serde_json::Value;
use tempfile::TempDir;

const STANDALONE: &str = include_str!("../../../fixtures/schema/valid/standalone.toml");

fn recite() -> Command {
    Command::new(env!("CARGO_BIN_EXE_recite"))
}

fn run(command: &mut Command) -> io::Result<Output> {
    command.output()
}

fn write_file(root: &Path, name: &str, source: &str) -> io::Result<PathBuf> {
    let path = root.join(name);
    fs::write(&path, source)?;
    Ok(path)
}

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn failure(output: &Output) {
    assert!(!output.status.success(), "CLI unexpectedly succeeded");
}

fn records(output: &Output) -> Result<Vec<Value>, serde_json::Error> {
    output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(serde_json::from_slice)
        .collect()
}

#[test]
fn exports_canonical_schema_from_toml_and_json() -> Result<(), Box<dyn Error>> {
    let temp = TempDir::new()?;
    let source = write_file(temp.path(), "schema.toml", STANDALONE)?;
    let output = temp.path().join("schema.json");
    success(&run(recite()
        .arg("export-schema")
        .arg("--schema")
        .arg(&source)
        .arg("--output")
        .arg(&output))?);
    let first = fs::read_to_string(&output)?;
    let loaded = load_schema_manifest_str("schema.json", &first);
    assert!(loaded.diagnostics.is_empty());
    assert!(loaded.schema.is_some());
    let second = temp.path().join("again.json");
    success(&run(recite()
        .arg("export-schema")
        .arg("--schema")
        .arg(&output)
        .arg("--output")
        .arg(&second))?);
    assert_eq!(first, fs::read_to_string(second)?);
    Ok(())
}

#[test]
fn native_producer_flags_are_paired_and_stamp_identity() -> Result<(), Box<dyn Error>> {
    let temp = TempDir::new()?;
    let source = write_file(temp.path(), "schema.toml", STANDALONE)?;
    let output = temp.path().join("native.json");
    success(&run(recite()
        .arg("export-schema")
        .arg("--schema")
        .arg(&source)
        .arg("--output")
        .arg(&output)
        .arg("--producer-kind")
        .arg("unity")
        .arg("--producer-id")
        .arg("asset-guid"))?);
    let json = fs::read_to_string(&output)?;
    let value: Value = serde_json::from_str(&json)?;
    assert_eq!(value["producer"]["kind"], "unity");
    assert_eq!(value["producer"]["id"], "asset-guid");
    assert_eq!(value["producer_fingerprints"][0]["kind"], "unity");
    assert_eq!(value["producer_fingerprints"][0]["id"], "asset-guid");
    assert!(json.contains("can_open"));
    failure(&run(recite()
        .arg("export-schema")
        .arg("--schema")
        .arg(&source)
        .arg("--output")
        .arg(&output)
        .arg("--producer-kind")
        .arg("godot"))?);
    assert_eq!(json, fs::read_to_string(&output)?);
    Ok(())
}

#[test]
fn structured_export_reports_artifact_and_preserves_output_on_failure() -> Result<(), Box<dyn Error>>
{
    let temp = TempDir::new()?;
    let source = write_file(temp.path(), "schema.toml", STANDALONE)?;
    let output = temp.path().join("schema.json");
    let result = run(recite()
        .arg("export-schema")
        .arg("--schema")
        .arg(&source)
        .arg("--output")
        .arg(&output)
        .arg("--output-format")
        .arg("structured")
        .arg("--invocation-id")
        .arg("export-1"))?;
    success(&result);
    assert!(result.stderr.is_empty());
    let result_records = records(&result)?;
    assert_eq!(result_records.len(), 2);
    assert_eq!(result_records[0]["event"], "command.started");
    assert_eq!(result_records[1]["event"], "command.result");
    assert_eq!(result_records[1]["status"], "success");
    assert_eq!(
        result_records[1]["data"]["artifact"]["size_bytes"],
        fs::metadata(&output)?.len()
    );

    let before = fs::read(&output)?;
    let invalid = write_file(temp.path(), "invalid.toml", "schema_version = 0\n")?;
    let result = run(recite()
        .arg("export-schema")
        .arg("--schema")
        .arg(invalid)
        .arg("--output")
        .arg(&output)
        .arg("--output-format")
        .arg("structured"))?;
    failure(&result);
    assert!(result.stderr.is_empty());
    let result_records = records(&result)?;
    assert_eq!(result_records[1]["status"], "content_diagnostics");
    assert!(
        result_records[1]["data"]["diagnostics"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert_eq!(before, fs::read(&output)?);

    failure(&run(recite()
        .arg("export-schema")
        .arg("--schema")
        .arg(&source)
        .arg("--output")
        .arg(&source))?);
    let hardlink = temp.path().join("schema-link.toml");
    fs::hard_link(&source, &hardlink)?;
    failure(&run(recite()
        .arg("export-schema")
        .arg("--schema")
        .arg(&source)
        .arg("--output")
        .arg(&hardlink))?);
    assert_eq!(fs::read_to_string(source)?, STANDALONE);
    Ok(())
}
