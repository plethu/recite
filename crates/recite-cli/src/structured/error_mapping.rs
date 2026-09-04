use std::path::Path;

use crate::error::CliError;
use crate::schema_inspection::machine_path;

use super::errors::{ErrorCategory, ErrorCode, ErrorDetails, StructuredError};

type ErrorParts<'a> = (
    ErrorCategory,
    ErrorCode,
    &'static str,
    Option<&'a Path>,
    Option<&'a Path>,
    Option<ErrorDetails>,
);

pub(super) fn structured_error(
    error: &CliError,
    fallback_operation: &'static str,
    fallback_path: Option<&Path>,
) -> StructuredError {
    let parts: ErrorParts<'_> = match error {
        CliError::Core(_) if matches!(fallback_operation, "run" | "trace") => generic(
            ErrorCategory::Fixture,
            ErrorCode::CoreValue,
            fallback_operation,
            fallback_path,
        ),
        CliError::Core(_) => compilation(ErrorCode::CoreValue, fallback_operation, fallback_path),
        CliError::Compile(_) => compilation(ErrorCode::Compile, fallback_operation, fallback_path),
        CliError::CompiledValue(_) => {
            compilation(ErrorCode::CompiledValue, fallback_operation, fallback_path)
        }
        CliError::DecodeAsset { path, .. } => asset(ErrorCode::DecodeAsset, "load_asset", path),
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
            "load_catalog",
            Some(path),
        ),
        CliError::DialogueCatalogPluralFormsConflict { path, .. } => localised(
            ErrorCode::DialogueCatalogPluralFormsConflict,
            "load_catalog",
            Some(path),
        ),
        CliError::DialogueCatalogMalformed { path, .. } => localised(
            ErrorCode::DialogueCatalogMalformed,
            "load_catalog",
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
        CliError::FixtureToml { path, .. } => fixture(ErrorCode::FixtureToml, "load_fixture", path),
        CliError::AssetMetadata { path, .. } => {
            asset(ErrorCode::AssetMetadata, "inspect_asset", path)
        }
        CliError::AssetNotFile { path } => asset(ErrorCode::AssetNotFile, "load_asset", path),
        CliError::Io(_) => generic(
            ErrorCategory::Io,
            ErrorCode::Io,
            fallback_operation,
            fallback_path,
        ),
        CliError::MalformedCompiledAsset { .. } => generic(
            ErrorCategory::Asset,
            ErrorCode::MalformedCompiledAsset,
            "load_asset",
            fallback_path,
        ),
        CliError::MissingPath(path) => generic(
            ErrorCategory::Input,
            ErrorCode::MissingPath,
            "resolve_path",
            Some(path),
        ),
        CliError::MissingFixtureChoice { prompt_keys } => fixture_details(
            ErrorCode::MissingFixtureChoice,
            fallback_path,
            ErrorDetails::MissingFixtureChoice {
                prompt_keys: prompt_keys.clone(),
            },
        ),
        CliError::NoInputs => input(ErrorCode::NoInputs, "collect_inputs", fallback_path),
        CliError::OutputOverwritesInput {
            output,
            input: related,
        } => (
            ErrorCategory::Input,
            ErrorCode::OutputOverwritesInput,
            "write_output",
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
        CliError::Read { path, .. } => {
            generic(ErrorCategory::Io, ErrorCode::Read, "read", Some(path))
        }
        CliError::ReadDir { path, .. } => generic(
            ErrorCategory::Io,
            ErrorCode::ReadDirectory,
            "read_directory",
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
            "acknowledge_effect",
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
        CliError::ProjectDiscovery { .. } => generic(
            ErrorCategory::Project,
            ErrorCode::ProjectDiscovery,
            fallback_operation,
            fallback_path,
        ),
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
        CliError::Write { path, .. } => {
            generic(ErrorCategory::Io, ErrorCode::Write, "write", Some(path))
        }
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

fn generic<'a>(
    category: ErrorCategory,
    code: ErrorCode,
    operation: &'static str,
    path: Option<&'a Path>,
) -> ErrorParts<'a> {
    (category, code, operation, path, None, None)
}

fn compilation<'a>(
    code: ErrorCode,
    operation: &'static str,
    path: Option<&'a Path>,
) -> ErrorParts<'a> {
    generic(ErrorCategory::Compilation, code, operation, path)
}

fn input<'a>(code: ErrorCode, operation: &'static str, path: Option<&'a Path>) -> ErrorParts<'a> {
    generic(ErrorCategory::Input, code, operation, path)
}

fn internal<'a>(
    code: ErrorCode,
    operation: &'static str,
    path: Option<&'a Path>,
) -> ErrorParts<'a> {
    generic(ErrorCategory::Internal, code, operation, path)
}

fn localised<'a>(
    code: ErrorCode,
    operation: &'static str,
    path: Option<&'a Path>,
) -> ErrorParts<'a> {
    generic(ErrorCategory::Localisation, code, operation, path)
}

fn localised_details<'a>(
    code: ErrorCode,
    operation: &'static str,
    path: Option<&'a Path>,
    details: ErrorDetails,
) -> ErrorParts<'a> {
    (
        ErrorCategory::Localisation,
        code,
        operation,
        path,
        None,
        Some(details),
    )
}

fn asset<'a>(code: ErrorCode, operation: &'static str, path: &'a Path) -> ErrorParts<'a> {
    generic(ErrorCategory::Asset, code, operation, Some(path))
}

fn fixture<'a>(code: ErrorCode, operation: &'static str, path: &'a Path) -> ErrorParts<'a> {
    generic(ErrorCategory::Fixture, code, operation, Some(path))
}

fn fixture_details<'a>(
    code: ErrorCode,
    path: Option<&'a Path>,
    details: ErrorDetails,
) -> ErrorParts<'a> {
    (
        ErrorCategory::Fixture,
        code,
        "select_fixture_choice",
        path,
        None,
        Some(details),
    )
}

fn unsupported<'a>(
    code: ErrorCode,
    operation: &'static str,
    path: Option<&'a Path>,
) -> ErrorParts<'a> {
    generic(ErrorCategory::Unsupported, code, operation, path)
}
