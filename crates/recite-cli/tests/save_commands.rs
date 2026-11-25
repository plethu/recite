#![cfg(test)]
//! `recite save impact` and `recite save inspect`.

use std::path::Path;
use std::process::Command;

use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::compiled::{
    CompiledAssetId, CompiledDialogue, CompilerVersion, SchemaFingerprint, SourceMapId,
    encode_compiled_dialogue_messagepack,
};
use recite_runtime::{
    DialogueEvent, EmptyDialogueContext, choose, next, snapshot::encode_session_messagepack,
    start_scene,
};
use tempfile::TempDir;

mod support;
use support::*;

const MANIFEST: &str = concat!(
    "format_version = 1\n\n",
    "[[scenes]]\n",
    "id = \"harbour\"\n",
    "asset = \"build/dialogue.recitec\"\n",
    "block = \"start\"\n",
    "participants = [\"narrator\"]\n",
);

const RELEASED: &str = concat!(
    ":: start default speaker=narrator\n",
    "> @arrive\n",
    "  The ferry docks.\n",
    "> @ask\n",
    "  Where to?\n",
    "  ? @town\n",
    "    Into town.\n",
    "    -> town\n",
    "  ? @stay\n",
    "    Stay aboard.\n",
    "    -> END\n",
    "\n",
    ":: town\n",
    "> @square\n",
    "  The square is quiet.\n",
    "! blocking open_map(town)\n",
    "> @fold\n",
    "  You fold the map.\n",
    "-> END\n",
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

/// A repository whose `v1` tag has the released project under `dialogue/`.
fn released_project() -> TempDir {
    let temp = TempDir::new().expect("tempdir");
    git(temp.path(), &["init", "-q", "-b", "main"]);
    std::fs::create_dir_all(temp.path().join("dialogue")).expect("project directory");
    std::fs::write(temp.path().join("dialogue/recite.project.toml"), MANIFEST).expect("manifest");
    write_recite(temp.path(), "dialogue/main.recite", RELEASED);
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-q", "-m", "release"]);
    git(temp.path(), &["tag", "v1"]);
    temp
}

fn compile(source: &str) -> CompiledDialogue {
    compile_inputs(
        [CompileInput::new("dialogue/main.recite", source)],
        CompileOptions::new(
            CompilerVersion::new("0.0.1").expect("compiler"),
            CompiledAssetId::new("build/dialogue.recitec").expect("asset id"),
            SourceMapId::new("build/dialogue.recitec.map").expect("map id"),
            SchemaFingerprint::NoSchema,
        ),
    )
    .expect("compiles")
    .asset
    .expect("asset")
    .dialogue
}

fn write_asset(path: &Path, dialogue: &CompiledDialogue) {
    std::fs::write(
        path,
        encode_compiled_dialogue_messagepack(dialogue).expect("encodes"),
    )
    .expect("writes asset");
}

#[test]
fn unchanged_project_resumes_every_save_exactly() {
    let temp = released_project();
    let output = recite()
        .args(["save", "impact", "--since", "v1", "--check"])
        .arg(temp.path().join("dialogue"))
        .assert_success();
    assert_eq!(
        stdout(&output),
        "build/dialogue.recitec: unchanged; all 7 save points restore exactly\n"
    );
}

#[test]
fn changed_effect_fails_the_check_until_its_block_is_acknowledged() {
    let temp = released_project();
    let dialogue = temp.path().join("dialogue");
    write_recite(
        temp.path(),
        "dialogue/main.recite",
        &RELEASED
            .replace("  The ferry docks.\n", "  The ferry finally docks.\n")
            .replace(
                "! blocking open_map(town)\n",
                "! blocking open_map(harbour)\n",
            ),
    );
    let output = recite()
        .args(["save", "impact", "--since", "v1", "--check"])
        .arg(&dialogue)
        .output()
        .expect("runs");
    output.assert_failure();
    let report = stdout(&output);
    assert!(
        report.starts_with("build/dialogue.recitec: 7 save points: 0 exact, 0 in unchanged blocks, 3 with edited text, 3 relocated, 1 falls back\n"),
        "{report}"
    );
    assert!(
        report.contains("main.recite:16:1: a save waiting for blocking effect `open_map` in block `town` falls back: a statement between @square and the save point was added, removed or edited"),
        "{report}"
    );
    assert!(
        stderr(&output).contains("1 save point falls back"),
        "{}",
        stderr(&output)
    );

    recite()
        .args([
            "save",
            "impact",
            "--since",
            "v1",
            "--check",
            "--allow-fallback",
            "town",
        ])
        .arg(&dialogue)
        .assert_success();
}

#[test]
fn json_report_lists_every_save_point_against_a_compiled_baseline() {
    let temp = TempDir::new().expect("tempdir");
    let baseline = temp.path().join("shipped.recitec");
    let candidate = temp.path().join("patched.recitec");
    write_asset(&baseline, &compile(RELEASED));
    write_asset(
        &candidate,
        &compile(&RELEASED.replace("> @arrive\n  The ferry docks.\n", "")),
    );
    let output = recite()
        .args(["save", "impact", "--format", "json", "--baseline"])
        .arg(&baseline)
        .arg(&candidate)
        .assert_success();
    let report: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("JSON report");
    assert_eq!(report["format_version"], 1);
    assert_eq!(report["unacknowledged_fallbacks"], 1);
    let asset = &report["assets"][0];
    assert_eq!(asset["asset_id"], "build/dialogue.recitec");
    assert_eq!(asset["summary"]["total"], 7);
    assert_eq!(asset["summary"]["fallback"], 1);
    let fallback = asset["save_points"]
        .as_array()
        .expect("save points")
        .iter()
        .find(|point| point["outcome"] == "fallback")
        .expect("one fallback");
    assert_eq!(fallback["kind"], "after_line");
    assert_eq!(fallback["line"], "arrive");
    assert_eq!(fallback["reason"], "landmark_removed");
    assert_eq!(fallback["acknowledged"], false);
}

#[test]
fn since_needs_a_project_directory() {
    let temp = TempDir::new().expect("tempdir");
    let asset = temp.path().join("shipped.recitec");
    write_asset(&asset, &compile(RELEASED));
    let output = recite()
        .args(["save", "impact", "--since", "v1"])
        .arg(&asset)
        .output()
        .expect("runs");
    output.assert_failure();
    assert!(
        stderr(&output).contains("is not a project directory"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn inspect_reads_a_save_and_predicts_its_restore() {
    let temp = TempDir::new().expect("tempdir");
    let shipped = compile(RELEASED);
    let mut session = start_scene(&shipped, None).expect("starts");
    while !matches!(
        next(&shipped, &mut session, &EmptyDialogueContext).expect("advances"),
        DialogueEvent::Prompt { .. }
    ) {}
    let save = temp.path().join("player.sav");
    std::fs::write(
        &save,
        encode_session_messagepack(&shipped, &session).expect("encodes save"),
    )
    .expect("writes save");
    let patched = temp.path().join("patched.recitec");
    write_asset(
        &patched,
        &compile(&RELEASED.replace("    Into town.\n", "    Head into town.\n")),
    );

    let output = recite()
        .args(["save", "inspect"])
        .arg(&save)
        .arg("--asset")
        .arg(&patched)
        .assert_success();
    let report = stdout(&output);
    assert!(
        report.starts_with("build/dialogue.recitec: saved in block `start` after @ask (snapshot format 1, compiler 0.0.1)\n  waiting at a prompt with choices @town, @stay\n"),
        "{report}"
    );
    assert!(report.contains(": resumes (text_edited)\n"), "{report}");

    let output = recite()
        .args(["save", "inspect", "--format", "json"])
        .arg(&save)
        .assert_success();
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("JSON");
    assert_eq!(document["block_id"], "start");
    assert_eq!(document["pending"]["kind"], "prompt");
    assert_eq!(document["position"]["landmark"]["kind"], "prompt");

    std::fs::write(&save, b"not a save").expect("overwrites save");
    let output = recite()
        .args(["save", "inspect"])
        .arg(&save)
        .output()
        .expect("runs");
    output.assert_failure();
    assert!(
        stderr(&output).contains("is not a readable Recite save"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn inspect_reports_that_an_ended_save_restores_no_session() {
    let temp = TempDir::new().expect("tempdir");
    let released = RELEASED.replace("> @ask\n", "! deferred remember(ferry)\n> @ask\n");
    let shipped = compile(&released);
    let mut session = start_scene(&shipped, None).expect("starts");
    while !matches!(
        next(&shipped, &mut session, &EmptyDialogueContext).expect("advances"),
        DialogueEvent::Prompt { .. }
    ) {}
    let end = choose(
        &shipped,
        &mut session,
        recite_core::ChoiceId::new("stay").expect("choice"),
        &EmptyDialogueContext,
    )
    .expect("chooses");
    assert!(matches!(end, DialogueEvent::End { .. }));
    let save = temp.path().join("player.sav");
    std::fs::write(
        &save,
        encode_session_messagepack(&shipped, &session).expect("encodes save"),
    )
    .expect("writes save");
    let shipped_path = temp.path().join("shipped.recitec");
    write_asset(&shipped_path, &shipped);
    let patched = temp.path().join("patched.recitec");
    write_asset(
        &patched,
        &compile(&released.replace("    Into town.\n", "    Head into town.\n")),
    );

    for asset in [&shipped_path, &patched] {
        let output = recite()
            .args(["save", "inspect"])
            .arg(&save)
            .arg("--asset")
            .arg(asset)
            .assert_success();
        let report = stdout(&output);
        assert!(
            report.contains("  the conversation had ended\n"),
            "{report}"
        );
        assert!(!report.contains("deferred effects"), "{report}");
        assert!(
            report.contains(": restores no session, because the conversation had ended\n"),
            "{report}"
        );
        let output = recite()
            .args(["save", "inspect", "--format", "json"])
            .arg(&save)
            .arg("--asset")
            .arg(asset)
            .assert_success();
        let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("JSON");
        assert_eq!(document["prediction"]["outcome"], "ended");
        assert_eq!(document["deferred_effects"], serde_json::json!([]));
    }
}

fn trusting(
    _: recite_runtime::ConditionQuery<'_>,
) -> Result<recite_runtime::ConditionValue, recite_runtime::ConditionEvaluationError> {
    Ok(recite_runtime::ConditionValue::Bool(true))
}

#[test]
fn inspect_names_the_line_before_a_branch_the_save_is_inside() {
    let temp = TempDir::new().expect("tempdir");
    let shipped = compile(concat!(
        ":: start default\n",
        "> @arrive\n",
        "  The ferry docks.\n",
        ":if trusts(player)\n",
        "  ! blocking open_map(town)\n",
        "  > @fold\n",
        "    You fold the map.\n",
        "-> END\n",
    ));
    let mut session = start_scene(&shipped, None).expect("starts");
    while !matches!(
        next(&shipped, &mut session, &trusting).expect("advances"),
        DialogueEvent::Effect(_)
    ) {}
    let save = temp.path().join("player.sav");
    std::fs::write(
        &save,
        encode_session_messagepack(&shipped, &session).expect("encodes save"),
    )
    .expect("writes save");

    let output = recite()
        .args(["save", "inspect"])
        .arg(&save)
        .assert_success();
    let report = stdout(&output);
    assert!(
        report.contains("saved in block `start` after @arrive"),
        "{report}"
    );
    let output = recite()
        .args(["save", "inspect", "--format", "json"])
        .arg(&save)
        .assert_success();
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("JSON");
    assert_eq!(document["position"]["landmark"]["line"], "arrive");
    assert_eq!(document["position"]["branch_depth"], 1);
}

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
