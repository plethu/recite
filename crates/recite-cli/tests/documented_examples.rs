#![cfg(test)]

use std::path::Path;

use tempfile::TempDir;

mod support;
use support::*;

fn blocks(document: &str, language: &str) -> Vec<String> {
    let fence = format!("```{language}\n");
    document
        .split(&fence)
        .skip(1)
        .map(|part| {
            part.split_once("\n```")
                .expect("closed example fence")
                .0
                .to_owned()
                + "\n"
        })
        .collect()
}

fn page(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs-site/src/content/docs")
            .join(name),
    )
    .expect("maintained documentation page")
}

#[test]
fn first_scene_commands_and_transcript_match_the_documented_source() {
    let document = page("getting-started/first-scene.md");
    let scenes = blocks(&document, "text");
    assert_eq!(scenes.len(), 2, "scene and full headless transcript");
    let fixtures = blocks(&document, "toml");
    assert_eq!(fixtures.len(), 1);
    let temp = TempDir::new().expect("tempdir");
    let source = write_file(temp.path(), "dialogue/crossroads.recite", &scenes[0]);
    run(recite().arg("validate").arg(&source))
        .assert_success()
        .assert_stderr("");
    run(recite().arg("extract").arg(&source))
        .assert_success()
        .assert_stdout_contains("msgctxt \"7701ceab59d2adfa057a\"");
    let asset = compile_project_asset(temp.path(), &source, "crossroads.recitec", None);
    let fixture = write_file(temp.path(), "fixture.toml", &fixtures[0]);
    run(recite()
        .arg("run")
        .arg(asset)
        .args(["--block", "which_way", "--fixture"])
        .arg(fixture))
    .assert_success()
    .assert_stderr("")
    .assert_stdout(&scenes[1]);
}

#[test]
fn native_migration_examples_compile_without_a_game_engine() {
    let temp = TempDir::new().expect("tempdir");
    let mut checked = 0;
    let directory =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs-site/src/content/docs/migration");
    let mut pages = std::fs::read_dir(directory)
        .expect("migration guides")
        .map(|entry| entry.expect("guide entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
        .collect::<Vec<_>>();
    pages.sort();
    for path in pages {
        let document = std::fs::read_to_string(path).expect("migration guide");
        for source in blocks(&document, "text")
            .into_iter()
            .filter(|source| source.trim_start().starts_with("::"))
        {
            let path = write_file(temp.path(), &format!("migration-{checked}.recite"), &source);
            compile_project_asset(
                temp.path(),
                &path,
                &format!("migration-{checked}.recitec"),
                None,
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 4,
        "maintained native migration targets remain covered"
    );
}
