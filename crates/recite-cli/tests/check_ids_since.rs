#![cfg(test)]

use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

mod support;
use support::*;

const RELEASED: &str = concat!(
    ":: start default\n",
    "> @greeting\n",
    "  Hello.\n",
    "  ? @ask\n",
    "    Ask.\n",
    "    -> END\n",
    "  ? @leave\n",
    "    Leave.\n",
    "    -> END\n",
);

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "user.name=Recite",
            "-c",
            "user.email=recite@example.invalid",
        ])
        .args(["-c", "commit.gpgsign=false", "-c", "tag.gpgsign=false"])
        .args(args)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?}");
}

/// A repository whose `v1` tag has `RELEASED` at `dialogue/scene.recite`.
fn released_project() -> TempDir {
    let temp = TempDir::new().expect("tempdir");
    git(temp.path(), &["init", "-q", "-b", "main"]);
    write_recite(temp.path(), "dialogue/scene.recite", RELEASED);
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-q", "-m", "release"]);
    git(temp.path(), &["tag", "v1"]);
    temp
}

#[test]
fn unchanged_and_moved_ids_pass_against_the_release() {
    let temp = released_project();
    let dialogue = temp.path().join("dialogue");
    recite()
        .args(["id", "check", "--since", "v1"])
        .arg(&dialogue)
        .assert_success();

    // Moving a line to another file keeps its identity.
    write_recite(
        temp.path(),
        "dialogue/scene.recite",
        ":: start default\n> @greeting\n  Hello.\n-> other.recite::other\n",
    );
    write_recite(
        temp.path(),
        "dialogue/other.recite",
        ":: other\n> @extra\n  Extra.\n  ? @ask\n    Ask.\n    -> END\n  ? @leave\n    Leave.\n    -> END\n",
    );
    recite()
        .args(["id", "check", "--since", "v1"])
        .arg(&dialogue)
        .assert_success();
}

#[test]
fn removed_and_renamed_ids_fail_until_acknowledged() {
    let temp = released_project();
    let dialogue = temp.path().join("dialogue");
    write_recite(
        temp.path(),
        "dialogue/scene.recite",
        &RELEASED
            .replace("  ? @leave\n    Leave.\n    -> END\n", "")
            .replace("@ask", "@ask_again"),
    );

    let output = run(recite()
        .args(["id", "check", "--since", "v1"])
        .arg(&dialogue));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("stable ID `@ask` from v1 is missing");
    output.assert_stderr_contains("stable ID `@leave` from v1 is missing");
    output.assert_stderr_not_contains("no diagnostics");

    let output = run(recite()
        .args(["id", "check", "--since", "v1", "--allow-removed", "leave"])
        .arg(&dialogue));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("`@ask`");
    output.assert_stderr_not_contains("`@leave`");

    recite()
        .args(["id", "check", "--since", "v1"])
        .args(["--allow-removed", "leave", "--allow-removed", "ask"])
        .arg(&dialogue)
        .assert_success();
}

#[test]
fn an_unavailable_release_ref_fails_instead_of_passing_on_an_empty_baseline() {
    let temp = released_project();
    let output = run(recite()
        .args(["id", "check", "--since", "v9"])
        .arg(temp.path().join("dialogue")));
    output.assert_failure();
    output.assert_stderr_contains("cannot read stable IDs at `v9`");
    output.assert_stderr_contains("fetch it (and its tags)");
}
