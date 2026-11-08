#![cfg(test)]

use std::fs;

use recite_core::{GENERATED_STABLE_ID_LEN, SourceId};
use tempfile::TempDir;

mod support;
use support::*;

const SCENE: &str = concat!(
    ":: start\n",
    "> speaker=mara @desk_greeting\n",
    "  Hello there.\n",
    "  ? @ask_room\n",
    "    I need a room.\n",
    "    -> END\n",
);

const FRENCH: &str = concat!(
    "msgctxt \"desk_greeting\"\nmsgid \"Hello there.\"\nmsgstr \"Bonjour.\"\n\n",
    "msgctxt \"desk_greeting&formal\"\nmsgid \"Hello there.\"\nmsgstr \"Bonjour, madame.\"\n\n",
    "#~ msgctxt \"retired_id\"\n#~ msgid \"Gone.\"\n#~ msgstr \"Parti.\"\n",
);

fn project() -> TempDir {
    let temp = TempDir::new().expect("tempdir");
    write_project_manifest(temp.path(), "[discovery]\ncatalogs = [\"locales/*.po\"]\n");
    write_recite(temp.path(), "dialogue/scene.recite", SCENE);
    write_file(temp.path(), "locales/fr.po", FRENCH);
    temp
}

fn read(temp: &TempDir, relative: &str) -> String {
    fs::read_to_string(temp.path().join(relative)).expect("read project file")
}

#[test]
fn rename_uses_manifest_sources_and_catalogues() {
    let temp = project();
    recite()
        .current_dir(temp.path())
        .args(["id", "rename", "@desk_greeting", "front_desk_hello"])
        .assert_success()
        .assert_stderr(
            "renamed `@desk_greeting` to `@front_desk_hello` and 2 catalogue contexts\n",
        );

    assert_eq!(
        read(&temp, "dialogue/scene.recite"),
        SCENE.replace("@desk_greeting", "@front_desk_hello")
    );
    assert_eq!(
        read(&temp, "locales/fr.po"),
        FRENCH.replace("\"desk_greeting", "\"front_desk_hello")
    );
}

#[test]
fn rename_with_explicit_paths_and_catalogue_by_location() {
    let temp = TempDir::new().expect("tempdir");
    let scene = write_recite(temp.path(), "scene.recite", SCENE);
    let catalogue = write_file(temp.path(), "fr.po", FRENCH);
    let target = format!("{}:4", scene.display());
    recite()
        .args(["id", "rename", &target, "room_request"])
        .arg(&scene)
        .arg("--catalog")
        .arg(&catalogue)
        .assert_success()
        .assert_stderr("renamed `@ask_room` to `@room_request` and 0 catalogue contexts\n");
    assert!(
        fs::read_to_string(&scene)
            .expect("scene")
            .contains("  ? @room_request\n")
    );
    assert_eq!(fs::read_to_string(&catalogue).expect("catalogue"), FRENCH);
}

#[test]
fn rename_without_catalogues_says_so() {
    let temp = TempDir::new().expect("tempdir");
    let scene = write_recite(temp.path(), "scene.recite", SCENE);
    recite()
        .args(["id", "rename", "ask_room", "room_request"])
        .arg(&scene)
        .assert_success()
        .assert_stderr(concat!(
            "renamed `@ask_room` to `@room_request` and 0 catalogue contexts\n",
            "no dialogue catalogues are configured; add [discovery].catalogs to recite.project.toml or pass --catalog\n",
        ));
}

#[test]
fn rename_refusals_leave_files_untouched() {
    let temp = project();
    for (new_id, expected) in [
        ("desk&formal", "contains `&`, which is reserved"),
        ("Desk", "stable ID `Desk` is malformed"),
        (
            "ask_room",
            "stable ID `@ask_room` is already used by a line or choice",
        ),
        ("retired_id", "still keys entries in catalogue"),
    ] {
        let output =
            run(recite()
                .current_dir(temp.path())
                .args(["id", "rename", "desk_greeting", new_id]));
        output.assert_failure();
        output.assert_stderr_contains(expected);
    }
    let output = run(recite()
        .current_dir(temp.path())
        .args(["id", "rename", "absent", "fresh"]));
    output.assert_failure();
    output.assert_stderr_contains("no line or choice carries stable ID `@absent`");
    assert_eq!(read(&temp, "dialogue/scene.recite"), SCENE);
    assert_eq!(read(&temp, "locales/fr.po"), FRENCH);
}

#[test]
fn reassign_repairs_one_side_of_a_duplicate() {
    let temp = project();
    let branch = ":: branch\n> @desk_greeting\n  Added on another branch.\n";
    write_recite(temp.path(), "dialogue/branch.recite", branch);

    let output = run(recite()
        .current_dir(temp.path())
        .args(["id", "reassign", "desk_greeting"]));
    output.assert_failure();
    output.assert_stderr_contains("select one as PATH:LINE");

    recite()
        .current_dir(temp.path())
        .args(["id", "reassign", "dialogue/branch.recite:2"])
        .assert_success()
        .assert_stderr_contains("catalogues stay with the remaining header");

    let rewritten = read(&temp, "dialogue/branch.recite");
    let field = rewritten
        .lines()
        .nth(1)
        .and_then(|line| line.strip_prefix("> "))
        .expect("header");
    let SourceId::Frozen(id) = SourceId::parse_field(field) else {
        panic!("`{field}` is not an ID field");
    };
    assert_eq!(id.as_str().len(), GENERATED_STABLE_ID_LEN);
    assert_eq!(read(&temp, "dialogue/scene.recite"), SCENE);
    assert_eq!(read(&temp, "locales/fr.po"), FRENCH);
}
