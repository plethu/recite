use crate::error::CliError;
use crate::i18n::{Messages, UiLocale};
use crate::structured::error_mapping::structured_error;
use crate::structured::errors::ErrorOperation;
use crate::watch::{
    ProjectBuildPreparationError, ProjectBuildPublisherError, TargetMapError, TargetPathError,
};
use recite_compiler::authoring::{BuildRequestError, BuildTarget, BuildTargetError};
use serde_json::{Value, json};
use std::path::Path;

#[test]
fn preparation_failures_keep_owned_operations_and_paths() {
    let cases = [
        (
            ProjectBuildPreparationError::Discovery(
                recite_config::ProjectDiscoveryError::NotFound {
                    start: "search".into(),
                },
            ),
            "project",
            "project_discovery",
            "discover_project",
            "discovery",
            "fallback.recite",
            None,
        ),
        (
            ProjectBuildPreparationError::Read {
                path: "input.recite".into(),
                message: "denied".into(),
            },
            "io",
            "read",
            "read_project_input",
            "read",
            "input.recite",
            None,
        ),
        (
            ProjectBuildPreparationError::NoInputs,
            "input",
            "no_inputs",
            "collect_inputs",
            "no_inputs",
            "fallback.recite",
            None,
        ),
        (
            ProjectBuildPreparationError::InvalidSchemaPath {
                path: "schema.json".into(),
                reason: "parent component".into(),
            },
            "schema",
            "watch_preparation",
            "resolve_schema",
            "invalid_schema_path",
            "schema.json",
            None,
        ),
        (
            ProjectBuildPreparationError::SchemaOutsideProject {
                declared: "schema.json".into(),
                resolved: "external.json".into(),
            },
            "schema",
            "watch_preparation",
            "resolve_schema",
            "schema_outside_project",
            "schema.json",
            Some("external.json"),
        ),
        (
            ProjectBuildPreparationError::SchemaWithoutModel {
                path: "schema.json".into(),
            },
            "schema",
            "watch_preparation",
            "load_schema",
            "schema_without_model",
            "schema.json",
            None,
        ),
        (
            ProjectBuildPreparationError::Schema {
                message: "invalid field".into(),
            },
            "schema",
            "watch_preparation",
            "load_schema",
            "schema",
            "fallback.recite",
            None,
        ),
        (
            ProjectBuildPreparationError::InvalidInputKey {
                key: "../input.recite".into(),
                reason: "parent component".into(),
            },
            "input",
            "watch_preparation",
            "prepare_inputs",
            "invalid_input_key",
            "fallback.recite",
            None,
        ),
        (
            ProjectBuildPreparationError::Authoring {
                message: "invalid effect".into(),
            },
            "compilation",
            "watch_preparation",
            "validate_project",
            "authoring",
            "fallback.recite",
            None,
        ),
        (
            ProjectBuildPreparationError::Request(BuildRequestError::MultipleSchemaInputs),
            "compilation",
            "watch_preparation",
            "prepare_request",
            "request",
            "fallback.recite",
            None,
        ),
        (
            ProjectBuildPreparationError::Target(BuildTargetError::Empty),
            "input",
            "watch_preparation",
            "prepare_targets",
            "target",
            "fallback.recite",
            None,
        ),
    ];
    for (source, category, code, operation, kind, path, related) in cases {
        let lower_message = source.to_string();
        let error = CliError::WatchPreparation { source };
        let mut expected = json!({
            "category": category, "code": code, "operation": operation,
            "path": {"encoding": "utf8", "value": path},
            "details": {"type": "watch", "kind": kind},
        });
        if let Some(path) = related {
            expected["related_path"] = json!({"encoding": "utf8", "value": path});
        }
        assert_eq!(wire(&error), expected);
        assert_human(&error, &lower_message);
    }
}

#[test]
fn publisher_failures_distinguish_primary_destination_and_related_input() {
    let cases = [
        (
            TargetMapError::NoTargets,
            "input",
            "prepare_publisher",
            "no_targets",
            "fallback.recite",
            None::<&str>,
        ),
        (
            TargetMapError::ProjectRoot {
                path: "project".into(),
                message: "denied".into(),
            },
            "io",
            "resolve_project_root",
            "project_root",
            "project",
            None,
        ),
        (
            TargetMapError::DuplicateDestination {
                target: target("out.recitec"),
                existing: target("other.recitec"),
                path: "destination.recitec".into(),
            },
            "input",
            "validate_target",
            "duplicate_destination",
            "destination.recitec",
            None,
        ),
    ];
    for (source, category, operation, kind, path, related) in cases {
        let lower_message = source.to_string();
        let error = CliError::WatchPublisher {
            source: ProjectBuildPublisherError::Targets(source),
        };
        let mut expected = json!({
            "category": category, "code": "watch_publisher", "operation": operation,
            "path": {"encoding": "utf8", "value": path},
            "details": {"type": "watch", "kind": kind},
        });
        if let Some(path) = related {
            expected["related_path"] = json!({"encoding": "utf8", "value": path});
        }
        assert_eq!(wire(&error), expected);
        assert_human(&error, &lower_message);
    }
    let source = TargetMapError::AliasesInput {
        target: target("out.recitec"),
        input: "input.recite".into(),
    };
    let lower_message = source.to_string();
    let error = CliError::WatchPublisher {
        source: ProjectBuildPublisherError::Targets(source),
    };
    assert_eq!(
        wire(&error),
        json!({
            "category": "input", "code": "watch_publisher", "operation": "validate_target",
            "path": {"encoding": "utf8", "value": "fallback.recite"},
            "related_path": {"encoding": "utf8", "value": "input.recite"},
            "details": {"type": "watch_target", "kind": "aliases_input", "target": "out.recitec"},
        })
    );
    assert_human(&error, &lower_message);
}

#[test]
fn target_rejections_have_stable_reason_tokens_without_message_parsing() {
    let reasons = [
        (TargetPathError::Absolute, "absolute"),
        (TargetPathError::Parent, "parent"),
        (TargetPathError::EmptyOrCurrent, "empty_or_current"),
        (TargetPathError::PlatformAmbiguous, "platform_ambiguous"),
        (TargetPathError::OutsideProject, "outside_project"),
        (TargetPathError::Directory, "directory"),
        (
            TargetPathError::NonDirectoryComponent,
            "non_directory_component",
        ),
        (TargetPathError::SymlinkComponent, "symlink_component"),
        (TargetPathError::Inspection("denied".into()), "inspection"),
    ];
    for (reason, kind) in reasons {
        let lower_message = format!("invalid project target out.recitec: {reason}");
        let error = CliError::WatchPublisher {
            source: ProjectBuildPublisherError::Targets(TargetMapError::InvalidTarget {
                target: target("out.recitec"),
                reason,
            }),
        };
        assert_eq!(
            wire(&error),
            json!({
                "category": "input", "code": "watch_publisher", "operation": "validate_target",
                "path": {"encoding": "utf8", "value": "fallback.recite"},
                "details": {"type": "watch_target", "kind": kind, "target": "out.recitec"},
            })
        );
        assert_human(&error, &lower_message);
    }
}

fn target(value: &str) -> BuildTarget {
    BuildTarget::new(value).unwrap()
}

fn wire(error: &CliError) -> Value {
    serde_json::to_value(structured_error(
        error,
        ErrorOperation::Compile,
        Some(Path::new("fallback.recite")),
    ))
    .unwrap()
}

fn assert_human(error: &CliError, expected: &str) {
    assert_eq!(error.to_string(), expected);
    assert_eq!(
        error.to_user_message(&Messages::load(&UiLocale::default()).unwrap()),
        expected
    );
}
