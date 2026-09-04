#![cfg(test)]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;

mod support;
use support::{compile_project_asset, recite, write_file, write_project_manifest};

struct WatchProcess {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    records: Receiver<Value>,
}

impl WatchProcess {
    fn start(root: &std::path::Path, invocation_id: Option<&str>) -> Self {
        let mut command = recite();
        command
            .arg("watch")
            .arg("--output-format")
            .arg("structured");
        if let Some(invocation_id) = invocation_id {
            command.arg("--invocation-id").arg(invocation_id);
        }
        command
            .arg(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().expect("watch process starts");
        let stdout = child.stdout.take().expect("watch stdout");
        let (sender, records) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let line = line.expect("watch stdout line");
                sender
                    .send(serde_json::from_str(&line).expect("watch JSON record"))
                    .expect("record receiver remains live");
            }
        });
        let stdin = child.stdin.take().expect("watch stdin");
        Self {
            child: Some(child),
            stdin: Some(stdin),
            records,
        }
    }

    fn next(&self) -> Value {
        self.records
            .recv_timeout(Duration::from_secs(5))
            .expect("watch record arrives")
    }

    fn try_next(&self, timeout: Duration) -> Option<Value> {
        self.records.recv_timeout(timeout).ok()
    }

    fn cancel(mut self) -> Vec<Value> {
        let stdin = self.stdin.as_mut().expect("watch stdin");
        writeln!(
            stdin,
            r#"{{"version":1,"command":"watch","action":"cancel"}}"#
        )
        .expect("send cancellation");
        stdin.flush().expect("flush cancellation");
        let mut records = Vec::new();
        loop {
            let record = self.next();
            let stopped = event(&record) == "watch.stopped";
            records.push(record);
            if stopped {
                break;
            }
        }
        let output = self
            .child
            .take()
            .expect("watch child remains owned")
            .wait_with_output()
            .expect("watch exits");
        assert!(output.status.success(), "watch stderr: {:?}", output.stderr);
        assert!(output.stderr.is_empty());
        records
    }

    fn close_stdin(&mut self) {
        self.stdin.take();
    }
}

impl Drop for WatchProcess {
    fn drop(&mut self) {
        let Some(child) = self.child.as_mut() else {
            return;
        };
        if child.try_wait().ok().flatten().is_none() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn project(root: &std::path::Path) {
    write_project_manifest(
        root,
        r#"[[scenes]]
id = "scene.start"
asset = "compiled/dialogue.recitec"
block = "start"
participants = ["hazel"]
"#,
    );
    write_file(
        root,
        "dialogue/main.recite",
        ":: start default speaker=hazel\n> intro@11111111111111111111\n  Hello.\n-> END\n",
    );
}

fn event(record: &Value) -> &str {
    record["event"].as_str().expect("event")
}

fn receive_until(process: &WatchProcess, expected: &str) -> Value {
    loop {
        let record = process.next();
        if event(&record) == expected {
            return record;
        }
    }
}

#[test]
fn structured_watch_streams_initial_success_and_flushes_before_idle() {
    let temp = TempDir::new().expect("tempdir");
    project(temp.path());
    let process = WatchProcess::start(temp.path(), Some("watch-1"));
    let started = process.next();
    assert_eq!(event(&started), "watch.started");
    assert_eq!(started["version"], 1);
    assert_eq!(started["sequence"], 0);
    assert_eq!(started["invocation_id"], "watch-1");
    let build_started = process.next();
    assert_eq!(event(&build_started), "watch.build.started");
    assert_eq!(build_started["sequence"], 1);
    assert_eq!(build_started["data"]["trigger"], "initial");
    let completed = process.next();
    assert_eq!(event(&completed), "watch.build.completed");
    assert_eq!(completed["sequence"], 2);
    assert_eq!(completed["data"]["status"], "succeeded");
    assert_eq!(completed["data"]["generation"], 0);
    assert_eq!(completed["data"]["snapshot_generation"], 0);
    assert_eq!(
        completed["data"]["inputs"],
        serde_json::json!(["dialogue/main.recite", "recite.project.toml"])
    );
    assert_eq!(
        completed["data"]["artifacts"][0]["path"]["encoding"],
        "utf8"
    );
    assert_eq!(completed["data"]["freshness"]["type"], "fresh");
    assert_eq!(
        completed["data"]["restart_guidance"],
        serde_json::json!({"type":"host_policy_required","decision":"unspecified"})
    );
    let waiting = process.next();
    assert_eq!(event(&waiting), "watch.waiting");
    assert_eq!(waiting["sequence"], 3);
    let records = process.cancel();
    let mut previous = waiting["sequence"].as_u64().expect("waiting sequence");
    for record in &records {
        let sequence = record["sequence"].as_u64().expect("record sequence");
        assert!(
            sequence > previous,
            "sequence is not monotonic: {records:?}"
        );
        assert_eq!(record["version"], 1);
        assert_eq!(record["invocation_id"], "watch-1");
        previous = sequence;
    }
    let cancel_index = records
        .iter()
        .position(|record| event(record) == "watch.cancel.requested")
        .expect("cancellation acknowledgement");
    let stopped_index = records
        .iter()
        .position(|record| event(record) == "watch.stopped")
        .expect("stop record");
    assert!(cancel_index < stopped_index);
}

#[test]
fn structured_watch_recovers_control_errors_and_requires_matching_invocation() {
    let temp = TempDir::new().expect("tempdir");
    project(temp.path());
    let mut process = WatchProcess::start(temp.path(), Some("watch-1"));
    assert_eq!(
        event(&receive_until(&process, "watch.waiting")),
        "watch.waiting"
    );
    for control in [
        "not-json\n".to_owned(),
        r#"{"version":2,"command":"watch","action":"cancel"}"#.to_owned() + "\n",
        r#"{"version":1,"command":"watch","action":"cancel","invocation_id":"other"}"#.to_owned()
            + "\n",
    ] {
        process
            .stdin
            .as_mut()
            .expect("watch stdin")
            .write_all(control.as_bytes())
            .expect("control");
        process
            .stdin
            .as_mut()
            .expect("watch stdin")
            .flush()
            .expect("flush control");
        let error = receive_until(&process, "watch.control.error");
        let expected = if control.starts_with("not") {
            "malformed"
        } else if control.contains("\"version\":2") {
            "unsupported_version"
        } else {
            "invocation_mismatch"
        };
        assert_eq!(error["data"]["error"]["type"], expected);
    }
    process.cancel();
}

#[test]
fn structured_watch_eof_is_not_cancellation() {
    let temp = TempDir::new().expect("tempdir");
    project(temp.path());
    let mut process = WatchProcess::start(temp.path(), None);
    receive_until(&process, "watch.waiting");
    process.close_stdin();
    assert!(process.try_next(Duration::from_millis(400)).is_none());

    write_file(
        temp.path(),
        "dialogue/main.recite",
        ":: start default speaker=hazel\n> intro@11111111111111111111\n  Still live.\n-> END\n",
    );
    assert_eq!(
        event(&receive_until(&process, "watch.build.completed")),
        "watch.build.completed"
    );
}

#[test]
fn structured_watch_reports_invalid_source_without_replacing_asset() {
    let temp = TempDir::new().expect("tempdir");
    project(temp.path());
    let asset = temp.path().join("compiled/dialogue.recitec");
    compile_project_asset(
        temp.path(),
        &temp.path().join("dialogue/main.recite"),
        "compiled/dialogue.recitec",
        None,
    );
    let original = fs::read(&asset).expect("baseline asset");

    write_file(temp.path(), "dialogue/main.recite", ":: start default\n>\n");
    let process = WatchProcess::start(temp.path(), None);
    receive_until(&process, "watch.build.started");
    let completed = receive_until(&process, "watch.build.completed");
    assert_eq!(completed["data"]["status"], "failed");
    assert_eq!(completed["data"]["outcome"]["type"], "diagnostics");
    assert!(completed["data"]["snapshot_generation"].is_null());
    assert_eq!(
        completed["data"]["inputs"],
        serde_json::json!(["dialogue/main.recite", "recite.project.toml"])
    );
    assert_eq!(completed["data"]["diagnostics"][0]["code"], "RECITE_ID001");
    assert!(
        completed["data"]["artifacts"]
            .as_array()
            .expect("artifacts")
            .is_empty()
    );
    assert_eq!(fs::read(&asset).expect("asset remains"), original);
    process.cancel();
}

#[test]
fn structured_watch_rebuilds_relevant_sources_and_ignores_generated_outputs() {
    let temp = TempDir::new().expect("tempdir");
    project(temp.path());
    let process = WatchProcess::start(temp.path(), None);
    receive_until(&process, "watch.waiting");

    write_file(
        temp.path(),
        "dialogue/main.recite",
        ":: start default speaker=hazel\n> intro@11111111111111111111\n  Changed.\n-> END\n",
    );
    let build_started = receive_until(&process, "watch.build.started");
    assert_eq!(build_started["data"]["trigger"], "input_changed");
    let completed = receive_until(&process, "watch.build.completed");
    assert_eq!(completed["data"]["status"], "succeeded");
    assert_eq!(
        event(&receive_until(&process, "watch.waiting")),
        "watch.waiting"
    );

    fs::write(
        temp.path().join("compiled/dialogue.recitec"),
        b"generated output wakeup",
    )
    .expect("generated output");
    assert!(process.try_next(Duration::from_millis(450)).is_none());
    process.cancel();
}

#[test]
fn structured_watch_reports_invalid_schema_without_replacing_asset() {
    let temp = TempDir::new().expect("tempdir");
    project(temp.path());
    let baseline = WatchProcess::start(temp.path(), None);
    receive_until(&baseline, "watch.waiting");
    baseline.cancel();
    let asset = temp.path().join("compiled/dialogue.recitec");
    let original = fs::read(&asset).expect("baseline asset");
    let process = WatchProcess::start(temp.path(), None);
    receive_until(&process, "watch.waiting");

    write_project_manifest(
        temp.path(),
        r#"[project]
schema = "schema.json"

[[scenes]]
id = "scene.start"
asset = "compiled/dialogue.recitec"
block = "start"
participants = ["hazel"]
"#,
    );
    write_file(
        temp.path(),
        "schema.json",
        r#"{"schema_version":"invalid"}"#,
    );
    let completed = receive_until(&process, "watch.build.completed");
    assert_eq!(completed["data"]["status"], "failed");
    assert_eq!(completed["data"]["outcome"]["type"], "diagnostics");
    assert_eq!(
        completed["data"]["inputs"],
        serde_json::json!(["dialogue/main.recite", "recite.project.toml", "schema.json"])
    );
    assert!(completed["data"]["snapshot_generation"].is_null());
    assert!(
        completed["data"]["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .iter()
            .any(|diagnostic| diagnostic["code"]
                .as_str()
                .is_some_and(|code| code.starts_with("RECITE_SCHEMA")))
    );
    assert_eq!(fs::read(asset).expect("asset remains"), original);
    process.cancel();
}

#[test]
fn structured_watch_keeps_running_after_recoverable_preparation_error() {
    let temp = TempDir::new().expect("tempdir");
    write_project_manifest(
        temp.path(),
        r#"[project]
schema = "schema.json"

[[scenes]]
id = "scene.start"
asset = "compiled/dialogue.recitec"
block = "start"
participants = ["hazel"]
"#,
    );
    write_file(
        temp.path(),
        "dialogue/main.recite",
        ":: start default speaker=hazel\n> intro@11111111111111111111\n  Hello.\n-> END\n",
    );

    let process = WatchProcess::start(temp.path(), None);
    let completed = receive_until(&process, "watch.build.completed");
    assert_eq!(completed["data"]["outcome"]["type"], "operational_failure");
    assert_eq!(completed["data"]["error"]["category"], "io");
    assert_eq!(completed["data"]["error"]["code"], "read");
    assert_eq!(completed["data"]["error"]["path"]["encoding"], "utf8");
    assert_eq!(
        completed["data"]["error"]["path"]["value"],
        temp.path().join("schema.json").to_string_lossy().as_ref()
    );
    assert!(
        completed["data"]["inputs"]
            .as_array()
            .expect("inputs")
            .iter()
            .any(|input| input == "schema.json")
    );
    assert_eq!(
        event(&receive_until(&process, "watch.waiting")),
        "watch.waiting"
    );

    write_file(
        temp.path(),
        "schema.json",
        r#"{"schema_version":"invalid"}"#,
    );
    let recovered = receive_until(&process, "watch.build.completed");
    assert_eq!(recovered["data"]["outcome"]["type"], "diagnostics");
    process.cancel();
}

#[path = "structured_watch/startup.rs"]
mod startup;
