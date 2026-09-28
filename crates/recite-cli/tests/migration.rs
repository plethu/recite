#![cfg(test)]

use std::fs;

use tempfile::TempDir;

mod support;
use support::*;

fn fixture(root: &std::path::Path, source: &str) -> std::path::PathBuf {
    write_file(root, "input.twee", source)
}

#[test]
fn whitespace_loss_requires_explicit_partial_write_acceptance() {
    for (family, source) in [
        ("twee", ":: Start\n  Hello.  \n"),
        ("ink", "=== Start ===\n  Hello.  \n-> END\n"),
        ("yarn", "title: Start\n---\nÉlodie: Hello.  \n===\n"),
    ] {
        let temp = TempDir::new().expect("tempdir");
        let input = write_file(temp.path(), &format!("input.{family}"), source);
        let destination = temp.path().join("converted");
        let output = run(recite()
            .arg("import")
            .arg(&input)
            .args(["--from", family, "--output-dir"])
            .arg(&destination));
        output.assert_failure();
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("report");
        assert_eq!(report["status"], "partial");
        assert!(!destination.exists());
        run(recite()
            .arg("import")
            .arg(&input)
            .args(["--from", family, "--accept-partial", "--output-dir"])
            .arg(&destination))
        .assert_success();
        run(recite()
            .arg("validate")
            .arg(destination.join("imported.recite")))
        .assert_success();
        assert_eq!(fs::read_to_string(&input).expect("original source"), source);
    }
}

#[test]
fn inspection_is_read_only_and_complete_import_produces_native_source() {
    let temp = TempDir::new().expect("tempdir");
    let input = fixture(temp.path(), ":: Start\nHello.\n");
    let inspected = run(recite().arg("import").arg(&input).args(["--from", "twee"]));
    inspected.assert_success();
    let report: serde_json::Value = serde_json::from_slice(&inspected.stdout).expect("report");
    assert_eq!(report["status"], "complete");
    assert_eq!(fs::read_dir(temp.path()).expect("list").count(), 1);
    let destination = temp.path().join("converted");
    run(recite()
        .arg("import")
        .arg(&input)
        .args(["--from", "twee", "--output-dir"])
        .arg(&destination))
    .assert_success();
    assert_eq!(
        fs::read_to_string(destination.join("imported.recite")).expect("source"),
        report["source"].as_str().expect("text")
    );
    run(recite()
        .arg("validate")
        .arg(destination.join("imported.recite")))
    .assert_success();
    let asset = temp.path().join("dialogue.recitec");
    run(recite()
        .arg("compile")
        .arg(destination.join("imported.recite"))
        .arg("-o")
        .arg(&asset))
    .assert_success();
    let runtime_fixture = write_file(temp.path(), "fixture.toml", "");
    let block = report["mappings"][0]["generated_id"]
        .as_str()
        .expect("block");
    run(recite()
        .arg("run")
        .arg(asset)
        .arg("--block")
        .arg(block)
        .arg("--fixture")
        .arg(runtime_fixture))
    .assert_success()
    .assert_stdout_contains("Hello.");
}

#[test]
fn partial_import_requires_explicit_acceptance_and_never_overwrites() {
    let temp = TempDir::new().expect("tempdir");
    let input = fixture(
        temp.path(),
        ":: Start\nHello.\n:: Unsafe\n<<if $flag>>\nSecret.\n",
    );
    let destination = temp.path().join("converted");
    run(recite()
        .arg("import")
        .arg(&input)
        .args(["--from", "twee", "--output-dir"])
        .arg(&destination))
    .assert_failure();
    assert!(!destination.exists());
    run(recite()
        .arg("import")
        .arg(&input)
        .args(["--from", "twee", "--accept-partial", "--output-dir"])
        .arg(&destination))
    .assert_success();
    let before = fs::read(destination.join("imported.recite")).expect("source");
    run(recite()
        .arg("import")
        .arg(&input)
        .args(["--from", "twee", "--accept-partial", "--output-dir"])
        .arg(&destination))
    .assert_failure();
    assert_eq!(
        fs::read(destination.join("imported.recite")).expect("source"),
        before
    );
    assert!(!String::from_utf8(before).expect("text").contains("Secret."));
}

#[test]
fn invalid_output_is_inspectable_but_cannot_be_published_even_with_partial_flag() {
    let temp = TempDir::new().expect("tempdir");
    let input = fixture(temp.path(), ":: Start\nHello.\n[[Leave->Missing]]\n");
    let destination = temp.path().join("converted");
    let output = run(recite()
        .arg("import")
        .arg(&input)
        .args(["--from", "twee", "--accept-partial", "--output-dir"])
        .arg(&destination));
    output.assert_failure();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("report");
    assert_eq!(report["status"], "invalid");
    assert!(
        !report["native_diagnostics"]
            .as_array()
            .expect("diagnostics")
            .is_empty()
    );
    assert!(!destination.exists());
}

#[cfg(unix)]
#[test]
fn output_symlink_is_rejected_without_touching_its_target() {
    let temp = TempDir::new().expect("tempdir");
    let input = fixture(temp.path(), ":: Start\nHello.\n");
    let destination = temp.path().join("output");
    std::os::unix::fs::symlink(temp.path(), &destination).expect("symlink");
    run(recite()
        .arg("import")
        .arg(&input)
        .args(["--from", "twee", "--output-dir"])
        .arg(&destination))
    .assert_failure();
    assert!(!temp.path().join("imported.recite").exists());
}
