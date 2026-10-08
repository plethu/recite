use std::path::Path;

use crate::error::CliError;
use crate::schema_inspection::machine_path;

#[path = "error_mapping/tests.rs"]
#[cfg(test)]
mod tests;

use super::errors::{ErrorCategory, ErrorCode, ErrorDetails, ErrorOperation, StructuredError};

type ErrorParts<'a> = (
    ErrorCategory,
    ErrorCode,
    ErrorOperation,
    Option<&'a Path>,
    Option<&'a Path>,
    Option<ErrorDetails>,
);

#[path = "error_mapping/classification.rs"]
mod classification;
#[path = "error_mapping/project.rs"]
mod project;
#[path = "error_mapping/watch.rs"]
mod watch;
use classification::{
    asset, compilation, fixture, fixture_details, generic, input, internal, localised,
    localised_details, unsupported,
};

pub(crate) fn structured_error(
    error: &CliError,
    fallback_operation: ErrorOperation,
    fallback_path: Option<&Path>,
) -> StructuredError {
    let parts: ErrorParts<'_> = match error {
        CliError::Import(_) => input(ErrorCode::Import, fallback_operation, fallback_path),
        CliError::ImportJson(_) => input(ErrorCode::ImportJson, fallback_operation, fallback_path),
        CliError::Core(_)
            if matches!(
                fallback_operation,
                ErrorOperation::Run | ErrorOperation::Trace
            ) =>
        {
            generic(
                ErrorCategory::Fixture,
                ErrorCode::CoreValue,
                fallback_operation,
                fallback_path,
            )
        }
        CliError::Core(_) => compilation(ErrorCode::CoreValue, fallback_operation, fallback_path),
        CliError::Compile(_) => compilation(ErrorCode::Compile, fallback_operation, fallback_path),
        CliError::CompiledValue(_) => {
            compilation(ErrorCode::CompiledValue, fallback_operation, fallback_path)
        }
        CliError::DecodeAsset { path, .. } => {
            asset(ErrorCode::DecodeAsset, ErrorOperation::LoadAsset, path)
        }
        CliError::Diagnostics => {
            internal(ErrorCode::Diagnostics, fallback_operation, fallback_path)
        }
        CliError::DiagnosticRendering { .. } => internal(
            ErrorCode::DiagnosticRendering,
            fallback_operation,
            fallback_path,
        ),
        CliError::DialogueCatalogConflict { path, .. } => localised(
            ErrorCode::DialogueCatalogConflict,
            ErrorOperation::LoadCatalog,
            Some(path),
        ),
        CliError::DialogueCatalogPluralFormsConflict { path, .. } => localised(
            ErrorCode::DialogueCatalogPluralFormsConflict,
            ErrorOperation::LoadCatalog,
            Some(path),
        ),
        CliError::DialogueCatalogMalformed { path, .. } => localised(
            ErrorCode::DialogueCatalogMalformed,
            ErrorOperation::LoadCatalog,
            Some(path),
        ),
        CliError::DialogueCatalogMissingLocale => localised(
            ErrorCode::DialogueCatalogMissingLocale,
            fallback_operation,
            fallback_path,
        ),
        CliError::DialogueCatalogSpecInvalid { spec } => localised_details(
            ErrorCode::DialogueCatalogSpecInvalid,
            fallback_operation,
            fallback_path,
            ErrorDetails::CatalogSpec { spec: spec.clone() },
        ),
        CliError::DialogueLocaleInvalid { field, locale } => localised_details(
            ErrorCode::DialogueLocaleInvalid,
            fallback_operation,
            fallback_path,
            ErrorDetails::Locale {
                field,
                locale: locale.clone(),
            },
        ),
        CliError::DiagnosticCodeMalformed { .. } => input(
            ErrorCode::DiagnosticCodeMalformed,
            fallback_operation,
            fallback_path,
        ),
        CliError::DiagnosticCodeUnknown { .. } => input(
            ErrorCode::DiagnosticCodeUnknown,
            fallback_operation,
            fallback_path,
        ),
        CliError::FixtureChoiceIndexOutOfRange {
            index,
            choice_count,
            prompt_keys,
        } => fixture_details(
            ErrorCode::FixtureChoiceIndexOutOfRange,
            fallback_path,
            ErrorDetails::FixtureChoiceIndex {
                index: *index,
                choice_count: *choice_count,
                prompt_keys: prompt_keys.clone(),
            },
        ),
        CliError::FixtureChoiceNotInPrompt {
            choice,
            prompt_keys,
        } => fixture_details(
            ErrorCode::FixtureChoiceNotInPrompt,
            fallback_path,
            ErrorDetails::FixtureChoice {
                choice: choice.clone(),
                prompt_keys: prompt_keys.clone(),
            },
        ),
        CliError::AmbiguousFixtureChoice {
            block,
            prompt_count,
        } => fixture_details(
            ErrorCode::AmbiguousFixtureChoice,
            fallback_path,
            ErrorDetails::AmbiguousFixture {
                block: block.clone(),
                prompt_count: *prompt_count,
            },
        ),
        CliError::FixtureToml { path, .. } => {
            fixture(ErrorCode::FixtureToml, ErrorOperation::LoadFixture, path)
        }
        CliError::AssetMetadata { path, .. } => {
            asset(ErrorCode::AssetMetadata, ErrorOperation::InspectAsset, path)
        }
        CliError::AssetNotFile { path } => {
            asset(ErrorCode::AssetNotFile, ErrorOperation::LoadAsset, path)
        }
        CliError::Io(_) => generic(
            ErrorCategory::Io,
            ErrorCode::Io,
            fallback_operation,
            fallback_path,
        ),
        CliError::MalformedCompiledAsset { .. } => generic(
            ErrorCategory::Asset,
            ErrorCode::MalformedCompiledAsset,
            ErrorOperation::LoadAsset,
            fallback_path,
        ),
        CliError::MissingPath(path) => generic(
            ErrorCategory::Input,
            ErrorCode::MissingPath,
            ErrorOperation::ResolvePath,
            Some(path),
        ),
        CliError::InvalidProjectRoot(path) => generic(
            ErrorCategory::Input,
            ErrorCode::InvalidProjectRoot,
            ErrorOperation::ResolvePath,
            Some(path),
        ),
        CliError::MissingFixtureChoice { prompt_keys } => fixture_details(
            ErrorCode::MissingFixtureChoice,
            fallback_path,
            ErrorDetails::MissingFixtureChoice {
                prompt_keys: prompt_keys.clone(),
            },
        ),
        CliError::NoInputs => input(
            ErrorCode::NoInputs,
            ErrorOperation::CollectInputs,
            fallback_path,
        ),
        CliError::OutputOverwritesInput {
            output,
            input: related,
        } => (
            ErrorCategory::Input,
            ErrorCode::OutputOverwritesInput,
            ErrorOperation::WriteOutput,
            Some(output),
            Some(related),
            None,
        ),
        CliError::PlayEof { .. } => {
            unsupported(ErrorCode::PlayEof, fallback_operation, fallback_path)
        }
        CliError::PlayInvalidInput(_) => unsupported(
            ErrorCode::PlayInvalidInput,
            fallback_operation,
            fallback_path,
        ),
        CliError::PlayInterrupted => unsupported(
            ErrorCode::PlayInterrupted,
            fallback_operation,
            fallback_path,
        ),
        CliError::PlayTuiRequiresTerminal => unsupported(
            ErrorCode::PlayTuiRequiresTerminal,
            fallback_operation,
            fallback_path,
        ),
        CliError::Read { path, .. } => generic(
            ErrorCategory::Io,
            ErrorCode::Read,
            ErrorOperation::Read,
            Some(path),
        ),
        CliError::ReadDir { path, .. } => generic(
            ErrorCategory::Io,
            ErrorCode::ReadDirectory,
            ErrorOperation::ReadDirectory,
            Some(path),
        ),
        CliError::Runtime(_) => generic(
            ErrorCategory::Runtime,
            ErrorCode::Runtime,
            fallback_operation,
            fallback_path,
        ),
        CliError::Preview(_) => generic(
            ErrorCategory::Runtime,
            ErrorCode::Preview,
            fallback_operation,
            fallback_path,
        ),
        CliError::BlockingEffectNeedsAcknowledgement { effect } => (
            ErrorCategory::Runtime,
            ErrorCode::BlockingEffectNeedsAcknowledgement,
            ErrorOperation::AcknowledgeEffect,
            fallback_path,
            None,
            Some(ErrorDetails::BlockingEffect {
                effect: effect.clone(),
            }),
        ),
        CliError::Bench { .. } => unsupported(ErrorCode::Bench, fallback_operation, fallback_path),
        CliError::Benchmark(_) => generic(
            ErrorCategory::Benchmark,
            ErrorCode::Benchmark,
            fallback_operation,
            fallback_path,
        ),
        CliError::BenchJson(_) => generic(
            ErrorCategory::Serialization,
            ErrorCode::BenchJson,
            fallback_operation,
            fallback_path,
        ),
        CliError::TraceJson(_) => generic(
            ErrorCategory::Serialization,
            ErrorCode::TraceJson,
            fallback_operation,
            fallback_path,
        ),
        CliError::SchemaInspection(_) => generic(
            ErrorCategory::Schema,
            ErrorCode::SchemaInspection,
            fallback_operation,
            fallback_path,
        ),
        CliError::UserConfig { .. } => generic(
            ErrorCategory::Configuration,
            ErrorCode::UserConfig,
            fallback_operation,
            fallback_path,
        ),
        CliError::ProjectDiscovery { source } => generic(
            ErrorCategory::Project,
            ErrorCode::ProjectDiscovery,
            fallback_operation,
            source.manifest_path().or(fallback_path),
        ),
        CliError::ProjectSchema { source } => project::schema(source, fallback_path),
        CliError::UiCatalog { .. } => generic(
            ErrorCategory::Configuration,
            ErrorCode::UiCatalog,
            fallback_operation,
            fallback_path,
        ),
        CliError::Watch { .. } => generic(
            ErrorCategory::Watch,
            ErrorCode::Watch,
            fallback_operation,
            fallback_path,
        ),
        CliError::WatchPreparation { source } => watch::preparation(source, fallback_path),
        CliError::WatchPublisher { source } => watch::publisher(source, fallback_path),
        CliError::WatchCoordinator { .. } => generic(
            ErrorCategory::Watch,
            ErrorCode::WatchCoordinator,
            fallback_operation,
            fallback_path,
        ),
        CliError::WatchRecovery { .. } => generic(
            ErrorCategory::Watch,
            ErrorCode::WatchRecovery,
            fallback_operation,
            fallback_path,
        ),
        CliError::Write { path, .. } => generic(
            ErrorCategory::Io,
            ErrorCode::Write,
            ErrorOperation::Write,
            Some(path),
        ),
    };
    StructuredError {
        category: parts.0,
        code: parts.1,
        operation: parts.2,
        path: parts.3.map(machine_path),
        related_path: parts.4.map(machine_path),
        details: parts.5,
    }
}
