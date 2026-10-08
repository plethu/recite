use std::path::Path;

use super::super::errors::{ErrorCategory, ErrorCode, ErrorDetails, ErrorOperation};
use super::ErrorParts;

pub(super) fn preparation<'a>(
    source: &'a crate::watch::ProjectBuildPreparationError,
    fallback_path: Option<&'a Path>,
) -> ErrorParts<'a> {
    use crate::watch::ProjectBuildPreparationError;

    match source {
        ProjectBuildPreparationError::Discovery(source) => (
            ErrorCategory::Project,
            ErrorCode::ProjectDiscovery,
            ErrorOperation::DiscoverProject,
            source.manifest_path().or(fallback_path),
            None,
            Some(ErrorDetails::Watch { kind: "discovery" }),
        ),
        ProjectBuildPreparationError::Read { path, .. } => (
            ErrorCategory::Io,
            ErrorCode::Read,
            ErrorOperation::ReadProjectInput,
            Some(path),
            None,
            Some(ErrorDetails::Watch { kind: "read" }),
        ),
        ProjectBuildPreparationError::NoInputs => (
            ErrorCategory::Input,
            ErrorCode::NoInputs,
            ErrorOperation::CollectInputs,
            fallback_path,
            None,
            Some(ErrorDetails::Watch { kind: "no_inputs" }),
        ),
        ProjectBuildPreparationError::InvalidSchemaPath { path, .. } => (
            ErrorCategory::Schema,
            ErrorCode::WatchPreparation,
            ErrorOperation::ResolveSchema,
            Some(path),
            None,
            Some(ErrorDetails::Watch {
                kind: "invalid_schema_path",
            }),
        ),
        ProjectBuildPreparationError::SchemaOutsideProject { declared, resolved } => (
            ErrorCategory::Schema,
            ErrorCode::WatchPreparation,
            ErrorOperation::ResolveSchema,
            Some(declared),
            Some(resolved),
            Some(ErrorDetails::Watch {
                kind: "schema_outside_project",
            }),
        ),
        ProjectBuildPreparationError::SchemaWithoutModel { path } => (
            ErrorCategory::Schema,
            ErrorCode::WatchPreparation,
            ErrorOperation::LoadSchema,
            Some(path),
            None,
            Some(ErrorDetails::Watch {
                kind: "schema_without_model",
            }),
        ),
        ProjectBuildPreparationError::Schema { .. } => (
            ErrorCategory::Schema,
            ErrorCode::WatchPreparation,
            ErrorOperation::LoadSchema,
            fallback_path,
            None,
            Some(ErrorDetails::Watch { kind: "schema" }),
        ),
        ProjectBuildPreparationError::InvalidInputKey { .. } => (
            ErrorCategory::Input,
            ErrorCode::WatchPreparation,
            ErrorOperation::PrepareInputs,
            fallback_path,
            None,
            Some(ErrorDetails::Watch {
                kind: "invalid_input_key",
            }),
        ),
        ProjectBuildPreparationError::Authoring { .. } => (
            ErrorCategory::Compilation,
            ErrorCode::WatchPreparation,
            ErrorOperation::ValidateProject,
            fallback_path,
            None,
            Some(ErrorDetails::Watch { kind: "authoring" }),
        ),
        ProjectBuildPreparationError::Request(_) => (
            ErrorCategory::Compilation,
            ErrorCode::WatchPreparation,
            ErrorOperation::PrepareRequest,
            fallback_path,
            None,
            Some(ErrorDetails::Watch { kind: "request" }),
        ),
        ProjectBuildPreparationError::Target(_) => (
            ErrorCategory::Input,
            ErrorCode::WatchPreparation,
            ErrorOperation::PrepareTargets,
            fallback_path,
            None,
            Some(ErrorDetails::Watch { kind: "target" }),
        ),
        _ => (
            ErrorCategory::Project,
            ErrorCode::WatchPreparation,
            ErrorOperation::PrepareProject,
            fallback_path,
            None,
            Some(ErrorDetails::Watch { kind: "unknown" }),
        ),
    }
}

pub(super) fn publisher<'a>(
    source: &'a crate::watch::ProjectBuildPublisherError,
    fallback_path: Option<&'a Path>,
) -> ErrorParts<'a> {
    use crate::watch::{ProjectBuildPublisherError, TargetMapError};

    let source = match source {
        ProjectBuildPublisherError::Targets(source) => source,
        _ => {
            return (
                ErrorCategory::Project,
                ErrorCode::WatchPublisher,
                ErrorOperation::PreparePublisher,
                fallback_path,
                None,
                Some(ErrorDetails::Watch { kind: "unknown" }),
            );
        }
    };
    match source {
        TargetMapError::NoTargets => (
            ErrorCategory::Input,
            ErrorCode::WatchPublisher,
            ErrorOperation::PreparePublisher,
            fallback_path,
            None,
            Some(ErrorDetails::Watch { kind: "no_targets" }),
        ),
        TargetMapError::ProjectRoot { path, .. } => (
            ErrorCategory::Io,
            ErrorCode::WatchPublisher,
            ErrorOperation::ResolveProjectRoot,
            Some(path),
            None,
            Some(ErrorDetails::Watch {
                kind: "project_root",
            }),
        ),
        TargetMapError::InvalidTarget { target, reason } => (
            ErrorCategory::Input,
            ErrorCode::WatchPublisher,
            ErrorOperation::ValidateTarget,
            fallback_path,
            None,
            Some(ErrorDetails::WatchTarget {
                kind: target_reason(reason),
                target: target.as_str().to_owned(),
            }),
        ),
        TargetMapError::AliasesInput { target, input } => (
            ErrorCategory::Input,
            ErrorCode::WatchPublisher,
            ErrorOperation::ValidateTarget,
            fallback_path,
            Some(input),
            Some(ErrorDetails::WatchTarget {
                kind: "aliases_input",
                target: target.as_str().to_owned(),
            }),
        ),
        TargetMapError::DuplicateDestination { path, .. } => (
            ErrorCategory::Input,
            ErrorCode::WatchPublisher,
            ErrorOperation::ValidateTarget,
            Some(path),
            None,
            Some(ErrorDetails::Watch {
                kind: "duplicate_destination",
            }),
        ),
        _ => (
            ErrorCategory::Project,
            ErrorCode::WatchPublisher,
            ErrorOperation::PreparePublisher,
            fallback_path,
            None,
            Some(ErrorDetails::Watch { kind: "unknown" }),
        ),
    }
}

fn target_reason(reason: &crate::watch::TargetPathError) -> &'static str {
    use crate::watch::TargetPathError;

    match reason {
        TargetPathError::Absolute => "absolute",
        TargetPathError::Parent => "parent",
        TargetPathError::EmptyOrCurrent => "empty_or_current",
        TargetPathError::PlatformAmbiguous => "platform_ambiguous",
        TargetPathError::OutsideProject => "outside_project",
        TargetPathError::Directory => "directory",
        TargetPathError::NonDirectoryComponent => "non_directory_component",
        TargetPathError::SymlinkComponent => "symlink_component",
        TargetPathError::Inspection(_) => "inspection",
        _ => "unknown",
    }
}
