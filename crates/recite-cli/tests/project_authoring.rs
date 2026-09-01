#![cfg(test)]

use tempfile::TempDir;

mod support;
use support::*;

#[test]
fn validate_project_reports_discovered_source_diagnostics_with_project_key() {
    let temp = TempDir::new().expect("tempdir");
    write_project_manifest(temp.path(), "");
    write_recite(
        temp.path(),
        "dialogue/malformed.recite",
        ":: start\n:if broken(\n  prose without a statement header\n",
    );

    let output = run(recite().arg("validate-project").arg(temp.path()));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("error RECITE_PARSE013 dialogue/malformed.recite:2:12");
    output.assert_stderr_contains("error RECITE_PARSE001 dialogue/malformed.recite:3:3");
}

#[test]
fn validate_project_reports_deterministic_cross_file_diagnostics() {
    let temp = TempDir::new().expect("tempdir");
    write_project_manifest(temp.path(), "");
    write_recite(
        temp.path(),
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> line@11111111111111111111\n",
            "  Start.\n",
            "-> target.recite::missing\n",
        ),
    );
    write_recite(temp.path(), "dialogue/target.recite", ":: known\n");

    let first = run(recite().arg("validate-project").arg(temp.path()));
    let second = run(recite().arg("validate-project").arg(temp.path()));
    assert_diagnostic_failure(&first);
    assert_eq!(stderr(&first), stderr(&second));
    first.assert_stderr_contains(
        "error RECITE_VALIDATE007 dialogue/start.recite:4:1 unknown block reference `target.recite::missing`",
    );
}

#[test]
fn validate_project_stops_project_checks_after_non_utf8_source_discovery() {
    let temp = TempDir::new().expect("tempdir");
    write_project_manifest(temp.path(), "");
    write_recite(
        temp.path(),
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> line@11111111111111111111\n",
            "  Start.\n",
            "-> target.recite::missing\n",
        ),
    );
    let unreadable = temp.path().join("dialogue/target.recite");
    std::fs::write(&unreadable, [0xff, 0xfe]).expect("non-UTF-8 source");
    let unreadable = std::fs::canonicalize(unreadable).expect("canonical source path");

    let output = run(recite().arg("validate-project").arg(temp.path()));
    assert_diagnostic_failure(&output);
    let stderr = stderr(&output);
    assert!(stderr.contains("error RECITE_CONFIG115"), "{stderr}");
    assert!(
        stderr.contains(&format!("{}:1:1", unreadable.display())),
        "{stderr}"
    );
    assert!(
        stderr.contains("project source is not valid UTF-8"),
        "{stderr}"
    );
    assert!(
        !stderr.contains("RECITE_VALIDATE007"),
        "partial discovery must not run project-wide reference checks: {stderr}"
    );
}

#[test]
fn validate_project_ignores_malformed_excluded_source() {
    let temp = TempDir::new().expect("tempdir");
    write_project_manifest(temp.path(), "[discovery]\nexcludes = [\"excluded/**\"]\n");
    write_recite(
        temp.path(),
        "included.recite",
        ":: start default\n> line@11111111111111111111\n  Included.\n",
    );
    write_recite(
        temp.path(),
        "excluded/malformed.recite",
        ":: start\n:if broken(\n  malformed\n",
    );

    recite()
        .arg("validate-project")
        .arg(temp.path())
        .assert_success()
        .assert_stderr("");
}

#[test]
fn check_fresh_does_not_emit_discovered_source_diagnostics() {
    let temp = TempDir::new().expect("tempdir");
    write_project_manifest(temp.path(), "");
    write_recite(
        temp.path(),
        "dialogue/malformed.recite",
        ":: start\n:if broken(\n  malformed\n",
    );

    let validate = run(recite().arg("validate-project").arg(temp.path()));
    assert_diagnostic_failure(&validate);
    validate.assert_stderr_contains("RECITE_PARSE013 dialogue/malformed.recite:2:12");

    recite()
        .arg("check-fresh")
        .arg(temp.path())
        .assert_success()
        .assert_stderr("");
}
