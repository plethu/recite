use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tempfile::TempDir;

use super::build::{BuildStatus, build_once};
use super::events::WatchState;
use super::inputs::collect_project_sources;
fn write_file(root: &Path, name: &str, source: &str) -> PathBuf {
    let path = root.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(&path, source).expect("write file");
    path
}

fn write_project(root: &Path) {
    write_file(
        root,
        "recite.project.toml",
        r#"[[scenes]]
id = "scene.start"
asset = "compiled/dialogue.recitec"
block = "start"
participants = ["hazel"]
"#,
    );
}

fn write_project_with_schema(root: &Path) {
    write_file(
        root,
        "recite.project.toml",
        r#"[project]
schema = "schema.json"

[[scenes]]
id = "scene.start"
asset = "compiled/dialogue.recitec"
block = "start"
participants = ["hazel"]
"#,
    );
}

fn valid_source() -> &'static str {
    ":: start default speaker=hazel\n> intro\n  Hello.\n-> END\n"
}

#[test]
fn build_once_writes_manifest_assets_from_project_sources() {
    let temp = TempDir::new().expect("tempdir");
    write_project(temp.path());
    write_file(temp.path(), "dialogue/main.recite", valid_source());

    let mut stderr = Vec::new();
    let mut state = WatchState::new(temp.path().to_owned());
    let status = build_once(&mut state, &mut stderr).expect("build");

    assert_eq!(status, BuildStatus::Fresh { asset_count: 1 });
    assert!(temp.path().join("compiled/dialogue.recitec").is_file());
    assert_eq!(String::from_utf8(stderr).expect("stderr"), "");
}

#[test]
fn duplicate_manifest_asset_paths_are_built_once() {
    let temp = TempDir::new().expect("tempdir");
    write_file(
        temp.path(),
        "recite.project.toml",
        r#"[[scenes]]
id = "scene.start"
asset = "compiled/shared.recitec"
block = "start"
participants = ["hazel"]

[[scenes]]
id = "scene.also_start"
asset = "compiled/shared.recitec"
block = "start"
participants = ["hazel"]
"#,
    );
    write_file(temp.path(), "dialogue/main.recite", valid_source());

    let mut stderr = Vec::new();
    let mut state = WatchState::new(temp.path().to_owned());
    let status = build_once(&mut state, &mut stderr).expect("build");

    assert_eq!(status, BuildStatus::Fresh { asset_count: 1 });
    assert!(temp.path().join("compiled/shared.recitec").is_file());
}

#[test]
fn invalid_source_reports_diagnostics_without_overwriting_existing_asset() {
    let temp = TempDir::new().expect("tempdir");
    write_project(temp.path());
    write_file(temp.path(), "dialogue/main.recite", valid_source());

    let mut state = WatchState::new(temp.path().to_owned());
    let mut stderr = Vec::new();
    build_once(&mut state, &mut stderr).expect("initial build");
    let asset = temp.path().join("compiled/dialogue.recitec");
    let original = fs::read(&asset).expect("asset");

    write_file(
        temp.path(),
        "dialogue/main.recite",
        ":: start default\n> \n",
    );
    let mut stderr = Vec::new();
    let status = build_once(&mut state, &mut stderr).expect("invalid build");

    assert_eq!(status, BuildStatus::Diagnostics);
    assert_eq!(fs::read(asset).expect("asset unchanged"), original);
    let stderr = String::from_utf8(stderr).expect("stderr");
    assert!(stderr.contains("RECITE_"));
}

#[test]
fn invalid_schema_reports_diagnostics_without_overwriting_existing_asset() {
    let temp = TempDir::new().expect("tempdir");
    write_project(temp.path());
    write_file(temp.path(), "dialogue/main.recite", valid_source());

    let mut state = WatchState::new(temp.path().to_owned());
    let mut stderr = Vec::new();
    build_once(&mut state, &mut stderr).expect("initial build");
    let asset = temp.path().join("compiled/dialogue.recitec");
    let original = fs::read(&asset).expect("asset");

    write_project_with_schema(temp.path());
    write_file(temp.path(), "schema.json", r#"{"schema_version":"one"}"#);
    let mut stderr = Vec::new();
    let status = build_once(&mut state, &mut stderr).expect("invalid schema build");

    assert_eq!(status, BuildStatus::Diagnostics);
    assert_eq!(fs::read(asset).expect("asset unchanged"), original);
    let stderr = String::from_utf8(stderr).expect("stderr");
    assert!(stderr.contains("RECITE_SCHEMA"));
}

#[test]
fn fixing_invalid_source_allows_later_rebuild_to_recover() {
    let temp = TempDir::new().expect("tempdir");
    write_project(temp.path());
    write_file(
        temp.path(),
        "dialogue/main.recite",
        ":: start default\n> \n",
    );

    let mut state = WatchState::new(temp.path().to_owned());
    let mut stderr = Vec::new();
    assert_eq!(
        build_once(&mut state, &mut stderr).expect("invalid build"),
        BuildStatus::Diagnostics
    );

    write_file(temp.path(), "dialogue/main.recite", valid_source());
    let mut stderr = Vec::new();
    assert_eq!(
        build_once(&mut state, &mut stderr).expect("fixed build"),
        BuildStatus::Fresh { asset_count: 1 }
    );
    assert!(temp.path().join("compiled/dialogue.recitec").is_file());
}

#[test]
fn source_collection_skips_hidden_directories_and_target() {
    let temp = TempDir::new().expect("tempdir");
    let kept = write_file(temp.path(), "dialogue/main.recite", valid_source());
    write_file(temp.path(), ".hidden/hidden.recite", valid_source());
    write_file(temp.path(), "target/generated.recite", valid_source());

    let sources = collect_project_sources(temp.path()).expect("sources");

    assert_eq!(sources, vec![kept]);
}

#[test]
fn relevant_events_include_manifest_sources_and_schema_but_ignore_outputs() {
    let temp = TempDir::new().expect("tempdir");
    let mut state = WatchState::new(temp.path().to_owned());
    state.schema_path = Some(temp.path().join("schema.json"));

    assert!(state.is_relevant_path(&temp.path().join("recite.project.toml")));
    assert!(state.is_relevant_path(&temp.path().join("dialogue/main.recite")));
    assert!(state.is_relevant_path(&temp.path().join("schema.json")));
    assert!(!state.is_relevant_path(&temp.path().join("compiled/dialogue.recitec")));
    assert!(!state.is_relevant_path(&temp.path().join("compiled/.dialogue.recitec.42.tmp")));
    assert!(!state.is_relevant_path(&temp.path().join(".hidden/main.recite")));
    assert!(!state.is_relevant_path(&temp.path().join("target/main.recite")));
}

#[test]
fn synthetic_relevant_events_are_debounced_into_one_rebuild() {
    let temp = TempDir::new().expect("tempdir");
    let state = WatchState::new(temp.path().to_owned());
    let batches = debounced_rebuild_count(
        &state,
        &[
            (0, "dialogue/main.recite"),
            (20, "dialogue/main.recite"),
            (40, "compiled/dialogue.recitec"),
            (260, "recite.project.toml"),
            (600, "dialogue/other.recite"),
        ],
        Duration::from_millis(250),
    );

    assert_eq!(batches, 2);
}

fn debounced_rebuild_count(
    state: &WatchState,
    events: &[(u64, &str)],
    debounce: Duration,
) -> usize {
    let mut batches = 0;
    let mut active_until = None;
    for (millis, path) in events {
        if !state.is_relevant_path(&PathBuf::from(path)) {
            continue;
        }
        let now = Duration::from_millis(*millis);
        match active_until {
            Some(until) if now <= until => {
                active_until = Some(now + debounce);
            }
            _ => {
                batches += 1;
                active_until = Some(now + debounce);
            }
        }
    }
    batches
}
