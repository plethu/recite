use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use super::{WatchProcess, project, receive_until, recite};

#[test]
fn structured_watch_reports_fatal_startup_as_typed_stopped_record() {
    let temp = TempDir::new().expect("tempdir");
    let missing = temp.path().join("missing");
    let mut command = recite();
    let output = command
        .arg("watch")
        .arg("--output-format")
        .arg("structured")
        .arg(&missing)
        .output()
        .expect("watch process");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let records = String::from_utf8(output.stdout)
        .expect("utf8 records")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("record"))
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["event"], "watch.started");
    assert_eq!(records[1]["event"], "watch.stopped");
    assert_eq!(records[1]["data"]["reason"]["type"], "fatal");
    assert_eq!(records[1]["data"]["error"]["code"], "missing_path");
}

#[test]
fn structured_watch_reports_file_root_as_typed_stopped_record() {
    let temp = TempDir::new().expect("tempdir");
    let file = temp.path().join("project");
    fs::write(&file, "not a project directory").expect("file root");
    let mut command = recite();
    let output = command
        .arg("watch")
        .arg("--output-format")
        .arg("structured")
        .arg(&file)
        .output()
        .expect("watch process");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let records = String::from_utf8(output.stdout)
        .expect("utf8 records")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("record"))
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 2);
    assert_eq!(records[1]["event"], "watch.stopped");
    assert_eq!(records[1]["data"]["reason"]["type"], "fatal");
    assert_eq!(records[1]["data"]["error"]["category"], "input");
    assert_eq!(records[1]["data"]["error"]["code"], "invalid_project_root");
    assert_eq!(
        records[1]["data"]["error"]["path"]["value"],
        file.to_string_lossy().as_ref()
    );
}

#[cfg(unix)]
#[test]
fn structured_watch_preserves_non_utf8_machine_paths() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let parent = TempDir::new().expect("tempdir");
    let root = parent
        .path()
        .join(OsString::from_vec(b"project-\xff".to_vec()));
    fs::create_dir(&root).expect("project root");
    project(&root);
    let process = WatchProcess::start(&root, None);
    let started = process.next();
    assert_eq!(started["data"]["project_root"]["encoding"], "unix_bytes");
    let completed = receive_until(&process, "watch.build.completed");
    assert_eq!(
        completed["data"]["artifacts"][0]["path"]["encoding"],
        "unix_bytes"
    );
    process.cancel();
}
