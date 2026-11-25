//! `save impact --since` builds the baseline project from a git revision.

use super::*;

#[test]
fn since_builds_a_project_at_the_repository_root_or_through_a_relative_path() {
    let temp = TempDir::new().expect("tempdir");
    git(temp.path(), &["init", "-q", "-b", "main"]);
    std::fs::write(temp.path().join("recite.project.toml"), MANIFEST).expect("manifest");
    write_recite(temp.path(), "main.recite", RELEASED);
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-q", "-m", "release"]);
    git(temp.path(), &["tag", "v1"]);
    let output = recite()
        .args(["save", "impact", "--since", "v1", "--check", "."])
        .current_dir(temp.path())
        .assert_success();
    assert!(
        stdout(&output).contains("unchanged; all 7 save points"),
        "{}",
        stdout(&output)
    );

    let nested = released_project();
    let sibling = nested.path().join("tools");
    std::fs::create_dir_all(&sibling).expect("sibling directory");
    let output = recite()
        .args(["save", "impact", "--since", "v1", "--check", "../dialogue"])
        .current_dir(&sibling)
        .assert_success();
    assert!(
        stdout(&output).contains("unchanged; all 7 save points"),
        "{}",
        stdout(&output)
    );

    // Inside the project, the baseline is still the whole project.
    let chapters = nested.path().join("dialogue/chapters");
    std::fs::create_dir_all(&chapters).expect("project subdirectory");
    let output = recite()
        .args(["save", "impact", "--since", "v1", "--check"])
        .current_dir(&chapters)
        .assert_success();
    assert!(
        stdout(&output).contains("unchanged; all 7 save points"),
        "{}",
        stdout(&output)
    );
}

#[cfg(unix)]
#[test]
fn since_checks_out_symlinked_sources_and_unusual_file_names() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().expect("tempdir");
    git(temp.path(), &["init", "-q", "-b", "main"]);
    let project = temp.path().join("dialogue");
    std::fs::create_dir_all(&project).expect("project directory");
    std::fs::write(project.join("recite.project.toml"), MANIFEST).expect("manifest");
    write_recite(temp.path(), "dialogue/real.recite", RELEASED);
    symlink("real.recite", project.join("main.recite")).expect("links");
    std::fs::write(project.join("notes\ndraft.txt"), "notes").expect("notes");
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-q", "-m", "release"]);
    git(temp.path(), &["tag", "v1"]);

    let output = recite()
        .args(["save", "impact", "--since", "v1", "--check"])
        .arg(&project)
        .assert_success();
    assert!(
        stdout(&output).contains("unchanged; all 7 save points"),
        "{}",
        stdout(&output)
    );
}

#[cfg(unix)]
#[test]
fn since_checks_out_a_symlinked_schema() {
    use std::os::unix::fs::symlink;

    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/synthetic/tiny");
    let temp = TempDir::new().expect("tempdir");
    git(temp.path(), &["init", "-q", "-b", "main"]);
    let project = temp.path().join("dialogue");
    for file in [
        "recite.project.toml",
        "locales/en-US.po",
        "src/shard-000.recite",
        "src/shard-001.recite",
    ] {
        let destination = project.join(file);
        std::fs::create_dir_all(destination.parent().expect("parent")).expect("directory");
        std::fs::copy(fixture.join(file), destination).expect("copies fixture");
    }
    std::fs::create_dir_all(project.join("schema")).expect("schema directory");
    std::fs::copy(
        fixture.join("schema/synthetic.schema.json"),
        project.join("schema/real.schema.json"),
    )
    .expect("copies schema");
    symlink(
        "real.schema.json",
        project.join("schema/synthetic.schema.json"),
    )
    .expect("links schema");
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-q", "-m", "release"]);
    git(temp.path(), &["tag", "v1"]);

    let output = recite()
        .args(["save", "impact", "--since", "v1", "--check"])
        .arg(&project)
        .assert_success();
    assert!(
        stdout(&output).contains(": unchanged;"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn a_baseline_the_compiler_now_rejects_reports_that_revision() {
    let temp = TempDir::new().expect("tempdir");
    git(temp.path(), &["init", "-q", "-b", "main"]);
    std::fs::write(temp.path().join("recite.project.toml"), MANIFEST).expect("manifest");
    let shipped = RELEASED.replace(
        ":: town\n",
        ":: town\n:match mood()\n  :case tired\n    -> END\n  :case tired\n    -> END\n  :case _\n    -> END\n",
    );
    write_recite(temp.path(), "main.recite", &shipped);
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-q", "-m", "release"]);
    git(temp.path(), &["tag", "v1"]);
    // The patch removes the arm traversal never selected.
    let patched = shipped.replace("  :case tired\n    -> END\n  :case _\n", "  :case _\n");
    write_recite(temp.path(), "main.recite", &patched);

    let output = recite()
        .args(["save", "impact", "--since", "v1", "."])
        .current_dir(temp.path())
        .output()
        .expect("runs");
    output.assert_failure();
    let report = stderr(&output);
    assert!(report.contains("RECITE_VALIDATE052"), "{report}");
    assert!(
        report.contains("these diagnostics are from the project at `v1`"),
        "{report}"
    );
    // `save impact` and `watch` share the note, so it names no flag.
    let note = report
        .lines()
        .find(|line| line.contains("from the project at"))
        .expect("note");
    assert!(!note.contains("--"), "{note}");
    // The excerpt is the release's line, not the working tree's.
    let line = shipped
        .lines()
        .position(|line| line == "  :case tired")
        .expect("first arm")
        + 3;
    assert!(
        report.contains(&format!("{line} |   :case tired")),
        "{report}"
    );
}
