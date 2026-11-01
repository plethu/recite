#![cfg(test)]

use std::fs;

use recite_core::{GENERATED_STABLE_ID_LEN, SourceId};
use tempfile::TempDir;

mod support;
use support::*;

#[test]
fn assign_ids_appends_generated_ids_and_keeps_existing_ones() {
    let temp = TempDir::new().expect("tempdir");
    let scene = write_recite(
        temp.path(),
        "scene.recite",
        concat!(
            ":: start default\n",
            "> speaker=mara\n",
            "  Hello.\n",
            "  ? requires=(trusts(player))\n",
            "    Ask.\n",
            "    -> END\n",
            "  ? @keep_me\n",
            "    Leave.\n",
            "    -> END\n",
        ),
    );
    let other = write_recite(
        temp.path(),
        "nested/other.recite",
        ":: other\n>\n  Elsewhere.\n-> END\n",
    );

    recite()
        .arg("assign-ids")
        .arg(temp.path())
        .assert_success()
        .assert_stderr("assigned 3 stable IDs\n");

    let scene_source = fs::read_to_string(&scene).expect("scene");
    assert!(scene_source.contains("  ? @keep_me\n"));
    let headers = [
        (&scene_source, "> speaker=mara "),
        (&scene_source, "  ? requires=(trusts(player)) "),
    ];
    for (source, prefix) in headers {
        let line = source
            .lines()
            .find(|line| line.starts_with(prefix))
            .unwrap_or_else(|| panic!("header `{prefix}` in {source}"));
        let field = &line[prefix.len()..];
        let SourceId::Frozen(id) = SourceId::parse_field(field) else {
            panic!("`{field}` is not a trailing ID field");
        };
        assert_eq!(id.as_str().len(), GENERATED_STABLE_ID_LEN);
    }
    assert!(
        fs::read_to_string(&other)
            .expect("other")
            .starts_with(":: other\n> @")
    );

    recite().arg("check-ids").arg(temp.path()).assert_success();
    recite()
        .arg("assign-ids")
        .arg(temp.path())
        .assert_success()
        .assert_stderr("every line and choice already has a stable ID\n");
}

#[test]
fn assign_ids_leaves_files_untouched_when_an_id_is_malformed() {
    let temp = TempDir::new().expect("tempdir");
    let source = ":: start default\n>\n  Missing.\n> @Malformed\n  Needs an author.\n-> END\n";
    let scene = write_recite(temp.path(), "scene.recite", source);

    let output = run(recite().arg("assign-ids").arg(&scene));
    assert_diagnostic_failure(&output);
    output.assert_stderr_contains("RECITE_ID007");
    assert_eq!(fs::read_to_string(&scene).expect("scene"), source);
}
