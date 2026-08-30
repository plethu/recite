use std::fmt::Display;
use std::fs;
use std::path::Path;

use recite_cli::watch::ProjectBuildRequest;
use recite_compiler::BuildInputKind;
use tempfile::TempDir;

fn write_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        require(fs::create_dir_all(parent), "parent directory");
    }
    require(fs::write(path, contents), "file");
}

fn require<T, E: Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error}"),
    }
}

fn require_some<T>(value: Option<T>, context: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("{context}"),
    }
}

fn manifest(schema: &str) -> String {
    format!(
        "format_version = 1\n\n[discovery]\nsource_roots = [\"dialogue\"]\n\n[project]\nschema = \"{schema}\"\n\n[[scenes]]\nid = \"scene.start\"\nasset = \"compiled/dialogue.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n"
    )
}

fn source() -> &'static str {
    ":: start default speaker=hazel\n> intro@11111111111111111111\n  Hello.\n-> END\n"
}

fn write_project(root: &Path, schema: &str) {
    write_file(root, "recite.project.toml", &manifest(schema));
    write_file(root, "dialogue/main.recite", source());
}

#[test]
fn regular_schema_stays_contained_and_keeps_project_relative_key() {
    let temp = require(TempDir::new(), "tempdir");
    write_project(temp.path(), "schema.json");
    write_file(
        temp.path(),
        "schema.json",
        r#"{"schema_version":1,"speakers":{"hazel":{"display_name":"Hazel"}}}"#,
    );

    let preparation = require(ProjectBuildRequest::prepare(temp.path()), "preparation");
    let request = require_some(preparation.request(), "ready request");
    let schema = require_some(
        request
            .build_request()
            .inputs()
            .iter()
            .find(|input| input.kind() == &BuildInputKind::Schema),
        "schema input",
    );
    assert_eq!(schema.key().as_str(), "schema.json");
}

#[test]
fn missing_schema_remains_a_typed_read_error() {
    let temp = require(TempDir::new(), "tempdir");
    write_project(temp.path(), "schema.json");

    let error = ProjectBuildRequest::prepare(temp.path()).expect_err("missing schema error");
    assert!(matches!(
        error,
        recite_cli::watch::ProjectBuildPreparationError::Read { path, .. }
            if path == temp.path().join("schema.json")
    ));
}

#[test]
fn non_utf8_schema_remains_a_typed_read_error() {
    let temp = require(TempDir::new(), "tempdir");
    write_project(temp.path(), "schema.json");
    require(
        fs::write(temp.path().join("schema.json"), [0xff, 0xfe]),
        "non-UTF8 schema",
    );

    let error = ProjectBuildRequest::prepare(temp.path()).expect_err("non-UTF8 schema error");
    assert!(matches!(
        error,
        recite_cli::watch::ProjectBuildPreparationError::Read { path, .. }
            if path == temp.path().join("schema.json")
    ));
}

#[cfg(unix)]
#[test]
fn schema_symlink_to_inside_is_read_but_keeps_declared_key() {
    use std::os::unix::fs::symlink;

    let temp = require(TempDir::new(), "tempdir");
    write_project(temp.path(), "schema/link.json");
    write_file(
        temp.path(),
        "schema/actual.json",
        r#"{"schema_version":1,"speakers":{"hazel":{"display_name":"Hazel"}}}"#,
    );
    require(
        fs::create_dir_all(temp.path().join("schema")),
        "schema directory",
    );
    require(
        symlink(
            temp.path().join("schema/actual.json"),
            temp.path().join("schema/link.json"),
        ),
        "inside symlink",
    );

    let preparation = require(ProjectBuildRequest::prepare(temp.path()), "preparation");
    let request = require_some(preparation.request(), "ready request");
    let schema = require_some(
        request
            .build_request()
            .inputs()
            .iter()
            .find(|input| input.kind() == &BuildInputKind::Schema),
        "schema input",
    );
    assert_eq!(schema.key().as_str(), "schema/link.json");
    assert!(request.schema().is_some());
}

#[cfg(unix)]
#[test]
fn schema_symlink_to_outside_is_a_typed_preparation_error() {
    use std::os::unix::fs::symlink;

    let project = require(TempDir::new(), "project tempdir");
    let outside = require(TempDir::new(), "outside tempdir");
    write_project(project.path(), "schema/link.json");
    write_file(
        outside.path(),
        "schema.json",
        r#"{"schema_version":1,"speakers":{"hazel":{"display_name":"Hazel"}}}"#,
    );
    require(
        fs::create_dir_all(project.path().join("schema")),
        "schema directory",
    );
    let outside_schema = outside.path().join("schema.json");
    let declared = project.path().join("schema/link.json");
    require(symlink(&outside_schema, &declared), "outside symlink");
    let resolved = require(fs::canonicalize(&outside_schema), "canonical outside");

    let error = match ProjectBuildRequest::prepare(project.path()) {
        Err(error) => error,
        Ok(outcome) => panic!("unexpected preparation outcome: {outcome:?}"),
    };
    assert!(matches!(
        error,
        recite_cli::watch::ProjectBuildPreparationError::SchemaOutsideProject {
            declared: ref actual_declared,
            resolved: ref actual_resolved,
        } if actual_declared == &declared
            && actual_resolved == &resolved
    ));
}
