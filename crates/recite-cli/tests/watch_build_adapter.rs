use std::fmt::Display;
use std::fs;
use std::path::Path;

use recite_cli::watch::{ProjectBuildEngine, ProjectBuildPreparation, ProjectBuildRequest};
use recite_compiler::{
    BuildControl, BuildEngine, BuildInputAuthority, BuildInputKind, FreshnessStatus,
};
use recite_core::decode_compiled_dialogue_messagepack;
use tempfile::TempDir;

fn write_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        require(fs::create_dir_all(parent), "parent");
    }
    require(fs::write(path, contents), "file");
}

fn require<T, E: Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error}"),
    }
}

fn manifest(schema: Option<&str>) -> String {
    let schema = schema.map_or_else(String::new, |schema| {
        format!("\n[project]\nschema = \"{schema}\"\n")
    });
    format!(
        "format_version = 1\n\n[discovery]\nsource_roots = [\"dialogue\"]\nexcludes = [\"dialogue/excluded/**\"]\n{schema}\n[[scenes]]\nid = \"scene.start\"\nasset = \"compiled/dialogue.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n"
    )
}

fn valid_source() -> &'static str {
    ":: start default speaker=hazel\n> intro@11111111111111111111\n  Hello.\n-> END\n"
}

fn valid_source_with(block: &str, id: &str, default: bool) -> String {
    let default = if default { " default" } else { "" };
    format!(":: {block}{default} speaker=hazel\n> intro@{id}\n  Hello.\n-> END\n")
}

fn ready(root: &Path) -> ProjectBuildRequest {
    match require(ProjectBuildRequest::prepare(root), "preparation") {
        ProjectBuildPreparation::Ready(request) => *request,
        ProjectBuildPreparation::Rejected { diagnostics } => {
            panic!("unexpected diagnostics: {diagnostics:?}")
        }
        _ => panic!("unknown preparation outcome"),
    }
}

#[test]
fn preparation_captures_saved_manifest_schema_and_sorted_sources() {
    let temp = require(TempDir::new(), "tempdir");
    write_file(
        temp.path(),
        "recite.project.toml",
        &manifest(Some("schema.json")),
    );
    write_file(
        temp.path(),
        "schema.json",
        r#"{"schema_version":1,"speakers":{"hazel":{"display_name":"Hazel"}}}"#,
    );
    write_file(
        temp.path(),
        "dialogue/z.recite",
        &valid_source_with("z", "11111111111111111111", true),
    );
    write_file(
        temp.path(),
        "dialogue/a.recite",
        &valid_source_with("a", "22222222222222222222", false),
    );
    write_file(
        temp.path(),
        "dialogue/excluded/ignored.recite",
        "not source",
    );

    let request = ready(temp.path());
    let inputs = request.build_request().inputs();
    assert_eq!(inputs.len(), 4);
    assert!(
        inputs
            .iter()
            .all(|input| input.authority() == BuildInputAuthority::Saved)
    );
    assert!(
        inputs
            .iter()
            .any(|input| input.kind() == &BuildInputKind::Manifest)
    );
    assert!(
        inputs
            .iter()
            .any(|input| input.kind() == &BuildInputKind::Schema)
    );
    let source_keys = inputs
        .iter()
        .filter(|input| input.kind() == &BuildInputKind::Source)
        .map(|input| input.key().as_str())
        .collect::<Vec<_>>();
    assert_eq!(source_keys, ["dialogue/a.recite", "dialogue/z.recite"]);
    assert_eq!(request.targets()[0].asset_id(), "compiled/dialogue.recitec");
    let schema = match request.schema() {
        Some(schema) => schema,
        None => panic!("schema was not captured"),
    };
    assert_eq!(schema.schema_version, 1);
}

#[test]
fn engine_is_deterministic_and_keeps_candidates_in_memory() {
    let temp = require(TempDir::new(), "tempdir");
    write_file(
        temp.path(),
        "recite.project.toml",
        &manifest(Some("schema.json")),
    );
    write_file(
        temp.path(),
        "schema.json",
        r#"{"schema_version":1,"speakers":{"hazel":{"display_name":"Hazel"}}}"#,
    );
    write_file(temp.path(), "dialogue/main.recite", valid_source());
    let request = ready(temp.path());

    let mut first_engine = ProjectBuildEngine::new(&request);
    let mut second_engine = ProjectBuildEngine::new(&request);
    let control = BuildControl::new();
    let first_check = first_engine.check(request.build_request(), &control);
    let second_check = second_engine.check(request.build_request(), &control);
    assert_eq!(first_check, second_check);
    assert!(first_check.is_valid());
    assert_eq!(first_check.freshness().status(), FreshnessStatus::Unknown);
    assert_eq!(
        first_check.freshness().expected(),
        request.build_request().fingerprints()
    );
    let first = require(
        first_engine.build(request.build_request(), &control),
        "build",
    );
    let second = require(
        second_engine.build(request.build_request(), &control),
        "build",
    );
    assert_eq!(first, second);
    assert_eq!(first.len(), 1);
    let asset = require(
        decode_compiled_dialogue_messagepack(first[0].bytes()),
        "asset",
    );
    assert_eq!(asset.header.asset_id.as_str(), "compiled/dialogue.recitec");
    assert_eq!(
        asset.header.schema_fingerprint,
        request.schema().expect("schema").canonical_fingerprint()
    );
    assert!(!temp.path().join("compiled/dialogue.recitec").exists());
}

#[test]
fn cancellation_before_build_prevents_candidate_materialisation() {
    let temp = require(TempDir::new(), "tempdir");
    write_file(temp.path(), "recite.project.toml", &manifest(None));
    write_file(temp.path(), "dialogue/main.recite", valid_source());
    let request = ready(temp.path());
    let mut engine = ProjectBuildEngine::new(&request);
    let control = BuildControl::new();
    control.cancel();

    let candidates = require(
        engine.build(request.build_request(), &control),
        "cancelled build",
    );
    assert!(candidates.is_empty());
}

#[test]
fn malformed_source_and_schema_are_rejected_with_structured_diagnostics() {
    let source_temp = require(TempDir::new(), "tempdir");
    write_file(source_temp.path(), "recite.project.toml", &manifest(None));
    write_file(
        source_temp.path(),
        "dialogue/main.recite",
        ":: start default\n> \n",
    );
    let source = require(
        ProjectBuildRequest::prepare(source_temp.path()),
        "preparation",
    );
    assert!(matches!(source, ProjectBuildPreparation::Rejected { .. }));
    assert!(
        source
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.severity == recite_core::DiagnosticSeverity::Error)
    );

    let schema_temp = require(TempDir::new(), "tempdir");
    write_file(
        schema_temp.path(),
        "recite.project.toml",
        &manifest(Some("schema.json")),
    );
    write_file(schema_temp.path(), "schema.json", "{not json");
    write_file(schema_temp.path(), "dialogue/main.recite", valid_source());
    let schema = require(
        ProjectBuildRequest::prepare(schema_temp.path()),
        "preparation",
    );
    assert!(matches!(schema, ProjectBuildPreparation::Rejected { .. }));
    assert!(schema
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.code.category() == recite_core::DiagnosticCategory::Schema));
}

#[test]
fn repeated_preparation_has_the_same_request_identity() {
    let temp = require(TempDir::new(), "tempdir");
    write_file(temp.path(), "recite.project.toml", &manifest(None));
    write_file(temp.path(), "dialogue/main.recite", valid_source());

    let first = ready(temp.path());
    let second = ready(temp.path());
    assert_eq!(first, second);
    assert_eq!(
        first.build_request().fingerprints(),
        second.build_request().fingerprints()
    );
}
