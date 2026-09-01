#![cfg(test)]

use tempfile::TempDir;

mod support;
use support::*;

#[test]
fn missing_preview_condition_keeps_legacy_runtime_error_text() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            ":if relationship(player)\n",
            "  > line@11111111111111111111\n",
            "    Visible.\n",
            "-> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let fixture = write_file(temp.path(), "fixture.toml", "");
    let output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    output.assert_failure().assert_stderr(
        "error: condition `relationship` failed: fixture is missing condition `relationship(player)`\n",
    );
}

#[test]
fn preview_condition_wrong_type_keeps_legacy_runtime_error_text() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            ":match relationship(player)\n",
            "  :case trusted\n",
            "    > line@22222222222222222222\n",
            "      Trusted.\n",
            "-> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        "[conditions]\n\"relationship(player)\" = true\n",
    );
    let output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    output
        .assert_failure()
        .assert_stderr("error: condition `relationship` returned bool but runtime expected enum\n");
}
