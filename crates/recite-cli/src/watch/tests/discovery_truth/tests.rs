use std::fs;

use recite_compiler::BuildTelemetry;
use tempfile::TempDir;

use super::super::events::WatchState;
use super::{BuildStatus, build_once, valid_source, write_file, write_project};

#[test]
fn non_utf8_discovery_rejects_build_without_overwriting_existing_asset() {
    let temp = TempDir::new().expect("tempdir");
    write_project(temp.path());
    let source = write_file(temp.path(), "dialogue/main.recite", valid_source());

    let mut state = WatchState::new(temp.path().to_owned());
    let mut stderr = Vec::new();
    build_once(&mut state, &mut stderr).expect("initial build");
    let asset = temp.path().join("compiled/dialogue.recitec");
    let original = fs::read(&asset).expect("asset");

    fs::write(&source, [0xff, 0xfe]).expect("invalid UTF-8 source");
    let mut stderr = Vec::new();
    let status = build_once(&mut state, &mut stderr).expect("invalid UTF-8 build");

    assert_eq!(
        status,
        BuildStatus::Diagnostics {
            telemetry: BuildTelemetry::none(),
        }
    );
    assert_eq!(fs::read(asset).expect("asset unchanged"), original);
    let stderr = String::from_utf8(stderr).expect("stderr");
    assert!(stderr.contains("error RECITE_CONFIG115"), "{stderr}");
    assert!(
        stderr.contains(&format!("{}:1:1", source.display())),
        "{stderr}"
    );
    assert!(
        stderr.contains("project source is not valid UTF-8"),
        "{stderr}"
    );
}

#[test]
fn partial_discovery_reports_readable_parse_errors_without_publishing() {
    let temp = TempDir::new().expect("tempdir");
    write_project(temp.path());
    let source = write_file(temp.path(), "dialogue/main.recite", valid_source());

    let mut state = WatchState::new(temp.path().to_owned());
    let mut stderr = Vec::new();
    build_once(&mut state, &mut stderr).expect("initial build");
    let asset = temp.path().join("compiled/dialogue.recitec");
    let original = fs::read(&asset).expect("asset");

    write_file(
        temp.path(),
        "dialogue/malformed.recite",
        ":: start\n:if broken(\n  prose without a statement header\n",
    );
    fs::write(&source, [0xff, 0xfe]).expect("invalid UTF-8 source");
    let mut stderr = Vec::new();
    let status = build_once(&mut state, &mut stderr).expect("partial build");

    assert_eq!(
        status,
        BuildStatus::Diagnostics {
            telemetry: BuildTelemetry::none(),
        }
    );
    assert_eq!(fs::read(asset).expect("asset unchanged"), original);
    let stderr = String::from_utf8(stderr).expect("stderr");
    assert!(
        stderr.contains("error RECITE_PARSE013 dialogue/malformed.recite:2:12"),
        "{stderr}"
    );
    assert!(
        stderr.contains("error RECITE_PARSE001 dialogue/malformed.recite:3:3"),
        "{stderr}"
    );
    assert!(stderr.contains("error RECITE_CONFIG115"), "{stderr}");
}
