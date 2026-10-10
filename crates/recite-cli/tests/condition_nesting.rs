use std::error::Error;
use std::fs;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;
use wait_timeout::ChildExt;

#[test]
fn deeply_nested_conditions_return_complete_structured_failures_without_aborting()
-> Result<(), Box<dyn Error>> {
    for expression in [
        format!("{}ready()", "not ".repeat(10_000)),
        format!("{}ready(){}", "(".repeat(10_000), ")".repeat(10_000)),
        format!("{}ready(){}", "not (".repeat(5_000), ")".repeat(5_000)),
    ] {
        let (status, records, stderr) = validate(&expression)?;
        assert_eq!(status.code(), Some(1), "{stderr}");
        assert!(stderr.is_empty(), "{stderr}");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0]["event"], "command.started");
        assert_eq!(records[1]["event"], "command.result");
        assert_eq!(records[1]["status"], "content_diagnostics");
        let diagnostic = &records[1]["data"]["diagnostics"][0];
        assert_eq!(diagnostic["code"], "RECITE_PARSE013");
        assert_eq!(
            diagnostic["presentation"]["id"],
            "diagnostic-parse-013-nesting-limit"
        );
        assert_eq!(
            diagnostic["presentation"]["arguments"]["limit"]["value"],
            128
        );
    }
    Ok(())
}

#[test]
fn wide_boolean_conditions_and_call_arguments_validate_without_stack_growth()
-> Result<(), Box<dyn Error>> {
    for expression in [
        vec!["ready()"; 10_000].join(" and "),
        vec!["ready()"; 10_000].join(" or "),
        format!("ready({})", vec!["1"; 10_000].join(", ")),
    ] {
        let (status, records, stderr) = validate(&expression)?;
        assert!(status.success(), "{stderr}");
        assert!(stderr.is_empty(), "{stderr}");
        assert_eq!(records.len(), 2);
        assert_eq!(records[1]["status"], "success");
        assert_eq!(records[1]["data"]["diagnostics"], Value::Array(Vec::new()));
    }
    Ok(())
}

fn validate(expression: &str) -> Result<(ExitStatus, Vec<Value>, String), Box<dyn Error>> {
    let temp = TempDir::new()?;
    let source = temp.path().join("conditions.recite");
    fs::write(
        &source,
        format!(
            ":: start default\n:if {expression}\n  > line@11111111111111111111\n    Hello.\n-> END\n"
        ),
    )?;
    let stdout = tempfile::tempfile()?;
    let stderr = tempfile::tempfile()?;
    let child = Command::new(env!("CARGO_BIN_EXE_recite"))
        .arg("validate")
        .arg("--output-format")
        .arg("structured")
        .arg(source)
        .stdout(Stdio::from(stdout.try_clone()?))
        .stderr(Stdio::from(stderr.try_clone()?))
        .spawn()?;
    let mut child = ChildGuard(child);
    let status = child
        .0
        .wait_timeout(Duration::from_secs(10))?
        .ok_or("condition validation exceeded the subprocess deadline")?;
    let records = serde_json_lines(stdout)?;
    Ok((status, records, read_output(stderr)?))
}

fn read_output(mut file: fs::File) -> Result<String, std::io::Error> {
    use std::io::{Read, Seek};
    file.rewind()?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    Ok(text)
}

fn serde_json_lines(file: fs::File) -> Result<Vec<Value>, Box<dyn Error>> {
    read_output(file)?
        .lines()
        .map(|line| serde_json::from_str(line).map_err(Into::into))
        .collect()
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
