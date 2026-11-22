#![cfg(test)]
//! Inside a project, sources are keyed as the project keys them, whatever path the author passes,
//! and commands default to the project's sources and schema.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

mod support;
use support::*;

/// A copy of the realistic pack, whose scenes link across files with `src/...` targets.
fn v1_pack() -> (TempDir, PathBuf) {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path().join("dialogue");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/realistic/v1-pack"),
        &root,
    );
    (temp, root)
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create directory");
    for entry in fs::read_dir(from).expect("read fixture") {
        let entry = entry.expect("fixture entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copy fixture file");
        }
    }
}

fn validate_in(directory: &Path, args: &[&str]) -> std::process::Output {
    run(recite().current_dir(directory).arg("validate").args(args))
}

#[test]
fn cross_file_references_resolve_whatever_path_is_passed() {
    let (_temp, root) = v1_pack();
    let absolute = root.join("src").display().to_string();
    for (directory, args) in [
        (root.clone(), vec!["src"]),
        (root.clone(), vec!["."]),
        (root.clone(), vec!["./src"]),
        (root.clone(), vec![absolute.as_str()]),
        (root.join("src"), vec!["."]),
        (
            root.parent().expect("parent").to_owned(),
            vec!["dialogue/src"],
        ),
    ] {
        validate_in(&directory, &args)
            .assert_success()
            .assert_stderr("ok  5 files, no diagnostics\n");
    }
}

#[test]
fn one_file_is_validated_against_the_rest_of_its_project() {
    let (_temp, root) = v1_pack();
    validate_in(&root, &["src/arrival.recite"])
        .assert_success()
        .assert_stderr("ok  1 file, no diagnostics\n");

    fs::write(
        root.join("src/broken.recite"),
        ":: broken\n> @broken_line\n  Nowhere.\n-> src/arrival.recite::arival\n",
    )
    .expect("write broken scene");
    let output = validate_in(&root.join("src"), &["broken.recite"]);
    output.assert_stderr_contains("unknown block reference `src/arrival.recite::arival`");
    output.assert_stderr_contains("--> broken.recite:4:1");
    output.assert_stderr_contains("1 error in 1 file");
}

#[test]
fn commands_default_to_the_project_sources_and_schema() {
    let (_temp, root) = v1_pack();
    validate_in(&root, &[])
        .assert_success()
        .assert_stderr("ok  5 files, no diagnostics\n");
    run(recite()
        .current_dir(&root)
        .args(["compile", "-o", "build/dialogue.recitec"]))
    .assert_success();
    assert!(
        root.join("build/dialogue.recitec").is_file(),
        "creates build/"
    );
    run(recite()
        .current_dir(&root)
        .args(["extract", "-o", "dialogue.pot"]))
    .assert_success();
    assert!(root.join("dialogue.pot").is_file());
    run(recite()
        .current_dir(&root)
        .args(["validate", "--only", "metadata"]))
    .assert_success();
}

#[test]
fn compile_checks_the_declared_schema_without_a_flag() {
    let (_temp, root) = v1_pack();
    let arrival = root.join("src/arrival.recite");
    let source = fs::read_to_string(&arrival).expect("read arrival");
    fs::write(
        &arrival,
        source.replace(
            "! immediate play_sfx(carrier_tone)",
            "! immediate play_sfx(carrier_tone, loud)",
        ),
    )
    .expect("write arrival");
    let output =
        run(recite()
            .current_dir(&root)
            .args(["compile", "src", "-o", "build/dialogue.recitec"]));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_VALIDATE018");
}

#[test]
fn a_different_explicit_schema_is_used_with_a_warning() {
    let (_temp, root) = v1_pack();
    fs::copy(
        root.join("schema/realistic.schema.json"),
        root.join("other.schema.json"),
    )
    .expect("copy schema");
    run(recite()
        .current_dir(&root)
        .args(["validate", "--schema", "other.schema.json", "src"]))
    .assert_success()
    .assert_stderr_contains("warning: --schema other.schema.json is not the project schema");
}

#[test]
fn no_paths_outside_a_project_explains_what_to_pass() {
    let temp = TempDir::new().expect("tempdir");
    let output = run(recite().current_dir(temp.path()).arg("validate"));
    assert!(!output.status.success());
    output.assert_stderr_contains("no recite.project.toml found here or above");
}

#[test]
fn run_starts_at_the_block_of_the_scene_that_uses_the_asset() {
    let (_temp, root) = v1_pack();
    run(recite()
        .current_dir(&root)
        .args(["compile", "-o", "build/dialogue.recitec"]))
    .assert_success();
    run(recite().current_dir(&root).args([
        "run",
        "build/dialogue.recitec",
        "--fixture",
        "runtime-fixture.toml",
    ]))
    .assert_success();

    fs::copy(
        root.join("build/dialogue.recitec"),
        root.join("build/loose.recitec"),
    )
    .expect("copy asset");
    let loose = run(recite().current_dir(&root).args([
        "run",
        "build/loose.recitec",
        "--fixture",
        "runtime-fixture.toml",
    ]));
    assert!(!loose.status.success());
    loose.assert_stderr_contains("pass --block: no scene in recite.project.toml uses");

    let manifest = root.join("recite.project.toml");
    let text = fs::read_to_string(&manifest).expect("read manifest");
    fs::write(
        &manifest,
        format!(
            "{text}\n[[scenes]]\nid = \"return\"\nasset = \"build/dialogue.recitec\"\nblock = \"arrival_return\"\nparticipants = [\"operator\"]\n"
        ),
    )
    .expect("write manifest");
    let ambiguous = run(recite().current_dir(&root).args([
        "run",
        "build/dialogue.recitec",
        "--fixture",
        "runtime-fixture.toml",
    ]));
    assert!(!ambiguous.status.success());
    ambiguous.assert_stderr_contains("at arrival, arrival_return");
}

#[test]
fn check_fresh_reports_changed_and_added_sources_once_each() {
    let (_temp, root) = v1_pack();
    run(recite()
        .current_dir(&root)
        .args(["compile", "-o", "build/dialogue.recitec"]))
    .assert_success();
    run(recite()
        .current_dir(&root)
        .args(["project", "check-fresh", "."]))
    .assert_success();

    for (path, from, to) in [
        ("src/arrival.recite", "The case narrows", "The case turns"),
        ("src/objects.recite", "The", "That"),
    ] {
        let file = root.join(path);
        let text = fs::read_to_string(&file).expect("read source");
        fs::write(&file, text.replacen(from, to, 1)).expect("edit source");
    }
    fs::write(
        root.join("src/station.recite"),
        ":: station\n> @st1\n  Hello.\n-> END\n",
    )
    .expect("add source");

    let output = run(recite()
        .current_dir(&root)
        .args(["project", "check-fresh", "."]));
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.matches("RECITE_FRESH001").count(), 1, "{stderr}");
    output.assert_stderr_contains(
        "2 sources changed since it was built (src/arrival.recite, src/objects.recite)",
    );
    output.assert_stderr_contains("1 source was added since it was built (src/station.recite)");
    output.assert_stderr_contains("= help: run `recite compile` again");
}

#[test]
fn unknown_speakers_and_blocks_suggest_the_closest_name() {
    let (_temp, root) = v1_pack();
    fs::write(
        root.join("src/station.recite"),
        concat!(
            ":: station speaker=narator\n",
            "> speaker=operatr @st1\n",
            "  Hello.\n",
            "> speaker=stranger @st2\n",
            "  Who?\n",
            "-> src/arrival.recite::arival\n",
        ),
    )
    .expect("write scene");
    let output = validate_in(&root, &[]);
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("error[RECITE_VALIDATE051]: unknown speaker `narator`");
    output.assert_stderr_contains("= help: did you mean `narrator`?");
    output.assert_stderr_contains("= help: did you mean `operator`?");
    output.assert_stderr_contains("unknown speaker `stranger`");
    output.assert_stderr_contains("= help: declare the speaker in the schema, or correct the name");
    output.assert_stderr_contains("= help: did you mean `src/arrival.recite::arrival`?");
}

#[test]
fn speakers_are_free_without_a_schema_registry() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "scene.recite",
        ":: start default speaker=anyone\n> speaker=someone @a1\n  Hi.\n-> END\n",
    );
    run(recite().arg("validate").arg(source)).assert_success();
}
