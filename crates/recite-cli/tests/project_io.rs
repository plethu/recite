#![cfg(test)]

use std::fs;

use recite_core::decode_compiled_dialogue_messagepack;
use tempfile::TempDir;

mod support;
use support::*;

#[test]
fn directory_asset_path_is_a_host_error_not_missing_asset() {
    let temp = TempDir::new().expect("tempdir");
    fs::create_dir(temp.path().join("compiled.recitec")).expect("asset directory");
    write_project_manifest(
        temp.path(),
        r#"[[scenes]]
id = "scene.start"
asset = "compiled.recitec"
block = "start"
participants = ["hazel"]
"#,
    );

    let output = run(recite().arg("validate-project").arg(temp.path()));
    output.assert_failure().assert_exit_code(1);
    output.assert_stderr_contains("compiled asset path");
    output.assert_stderr_contains("is not a regular file");
    output.assert_stderr_not_contains("RECITE_PROJECT003");
}

#[test]
fn nul_asset_path_is_a_host_metadata_error_not_missing_asset() {
    let temp = TempDir::new().expect("tempdir");
    write_project_manifest(
        temp.path(),
        r#"[[scenes]]
id = "scene.start"
asset = "compiled\u0000.recitec"
block = "start"
participants = ["hazel"]
"#,
    );

    let output = run(recite().arg("validate-project").arg(temp.path()));
    output.assert_failure().assert_exit_code(1);
    output.assert_stderr_contains("failed to inspect compiled asset");
    output.assert_stderr_not_contains("RECITE_PROJECT003");
}

#[test]
fn invalid_utf8_source_is_a_read_error_not_missing_source_diagnostic() {
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
    fs::write(&source, [0xff, 0xfe]).expect("invalid UTF-8 source");

    let output = run(recite().arg("check-fresh").arg(temp.path()));
    output.assert_failure().assert_exit_code(1);
    output.assert_stderr_contains("failed to read");
    output.assert_stderr_not_contains("RECITE_PROJECT006");
}

#[test]
fn earlier_non_not_found_source_candidate_blocks_project_fallback() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(temp.path(), "dialogue.recite", project_source());
    let asset = compile_project_asset(temp.path(), &source, "compiled/dialogue.recitec", None);
    write_project_manifest(
        temp.path(),
        r#"[[scenes]]
id = "scene.start"
asset = "compiled/dialogue.recitec"
block = "start"
participants = ["hazel"]
"#,
    );

    let bytes = fs::read(&asset).expect("compiled asset");
    let decoded = decode_compiled_dialogue_messagepack(&bytes).expect("decode asset");
    let first_candidate = asset
        .parent()
        .expect("asset parent")
        .join(&decoded.sources[0].path);
    fs::create_dir_all(&first_candidate).expect("blocking source candidate directory");

    let output = run(recite().arg("check-fresh").arg(temp.path()));
    output.assert_failure().assert_exit_code(1);
    output.assert_stderr_contains("failed to read");
    output.assert_stderr_not_contains("RECITE_PROJECT006");
}
