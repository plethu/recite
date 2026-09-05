use std::path::Path;

use recite_compiler::{PotExtractionReport, compile_inputs, compile_inputs_with_schema};
use recite_core::ProjectSchema;

use crate::args::{Command, CompileArgs, ExtractArgs, RuntimeArgs, ValidateArgs};
use crate::error::CliError;
use crate::fs::{
    collect_input_files, compile_options, load_schema, read_compile_inputs_for_output,
    read_compile_inputs_from_files, reject_output_input_alias, validate_inputs, write_staged,
};
use crate::runtime_fixture::{
    RuntimeFixtureOptions, dialogue_preview_from_fixture, execute_runtime_fixture,
    load_compiled_asset, load_runtime_fixture, trace_document,
};

use super::data::{
    CatalogEntry, ContentDiagnosticData, StructuredOutcome, SuccessData, artifact_metadata,
    diagnostic_records,
};

pub(super) struct CommandFailure {
    pub(super) error: Box<CliError>,
    pub(super) operation: &'static str,
    pub(super) path: Option<std::path::PathBuf>,
}

impl CommandFailure {
    fn new(error: CliError, operation: &'static str, path: Option<std::path::PathBuf>) -> Self {
        Self {
            error: Box::new(error),
            operation,
            path,
        }
    }
}

pub(super) fn execute(command: Command) -> Result<StructuredOutcome, CommandFailure> {
    match command {
        Command::Validate(args) => {
            let path = args.paths.first().cloned();
            validate(args).map_err(|error| CommandFailure::new(error, "validate", path))
        }
        Command::Compile(args) => {
            let path = Some(args.output.clone());
            compile(args).map_err(|error| CommandFailure::new(error, "compile", path))
        }
        Command::Extract(args) => {
            let path = args.output.clone().or_else(|| args.paths.first().cloned());
            extract(args).map_err(|error| CommandFailure::new(error, "extract", path))
        }
        Command::Run(args) => {
            let asset = args.asset.clone();
            let fixture = args.fixture.clone();
            runtime(args, RuntimeFixtureOptions::default()).map_err(|error| {
                let path = runtime_failure_path(&error, &asset, &fixture);
                CommandFailure::new(error, "run", Some(path))
            })
        }
        Command::Trace(args) => {
            let asset = args.runtime.asset.clone();
            let fixture = args.runtime.fixture.clone();
            runtime(
                args.runtime,
                RuntimeFixtureOptions {
                    metrics: args.metrics,
                },
            )
            .map_err(|error| {
                let path = runtime_failure_path(&error, &asset, &fixture);
                CommandFailure::new(error, "trace", Some(path))
            })
        }
        _ => Err(CommandFailure::new(
            CliError::MalformedCompiledAsset {
                reason: "structured protocol was requested for an unsupported command".to_owned(),
            },
            "dispatch",
            None,
        )),
    }
}

fn runtime_failure_path(error: &CliError, asset: &Path, fixture: &Path) -> std::path::PathBuf {
    match error {
        CliError::Core(_)
        | CliError::DialogueCatalogMissingLocale
        | CliError::DialogueCatalogSpecInvalid { .. }
        | CliError::DialogueLocaleInvalid { .. }
        | CliError::FixtureChoiceIndexOutOfRange { .. }
        | CliError::FixtureChoiceNotInPrompt { .. }
        | CliError::AmbiguousFixtureChoice { .. }
        | CliError::MissingFixtureChoice { .. } => fixture.to_owned(),
        _ => asset.to_owned(),
    }
}

fn validate(args: ValidateArgs) -> Result<StructuredOutcome, CliError> {
    let diagnostics = validate_inputs(&args.paths, None)?.into_all();
    let records = diagnostic_records(&diagnostics)?;
    let valid = records.is_empty();
    Ok(if valid {
        StructuredOutcome::success(SuccessData::Validate {
            diagnostics: records,
        })
    } else {
        StructuredOutcome::content_diagnostics(ContentDiagnosticData::Validate {
            diagnostics: records,
        })
    })
}

fn compile(args: CompileArgs) -> Result<StructuredOutcome, CliError> {
    let input_files = collect_input_files(&args.paths)?;
    reject_output_input_alias(&args.output, &input_files)?;
    let inputs = read_compile_inputs_for_output(&args.output, input_files)?;
    let schema = match schema_or_diagnostics(args.schema.as_deref())? {
        SchemaLoad::Loaded(schema) => schema,
        SchemaLoad::Diagnostics(diagnostics) => {
            return Ok(StructuredOutcome::content_diagnostics(
                ContentDiagnosticData::Compile { diagnostics },
            ));
        }
    };
    let options = compile_options(&args.output, schema.as_deref())?;
    let report = match schema.as_deref() {
        Some(schema) => compile_inputs_with_schema(inputs, options, schema)?,
        None => compile_inputs(inputs, options)?,
    };
    let diagnostics = diagnostic_records(&report.diagnostics)?;
    let Some(asset) = report.asset else {
        return Ok(StructuredOutcome::content_diagnostics(
            ContentDiagnosticData::Compile { diagnostics },
        ));
    };

    write_artifact(&args.output, &asset.messagepack)?;
    let artifact = artifact_metadata(&args.output)?;
    Ok(StructuredOutcome::success(SuccessData::Compile {
        diagnostics,
        artifact,
    }))
}

fn extract(args: ExtractArgs) -> Result<StructuredOutcome, CliError> {
    let input_files = collect_input_files(&args.paths)?;
    if let Some(output) = &args.output {
        reject_output_input_alias(output, &input_files)?;
    }
    let inputs = read_compile_inputs_from_files(input_files)?;
    let schema = match schema_or_diagnostics(args.schema.as_deref())? {
        SchemaLoad::Loaded(schema) => schema,
        SchemaLoad::Diagnostics(diagnostics) => {
            return Ok(StructuredOutcome::content_diagnostics(
                ContentDiagnosticData::Extract { diagnostics },
            ));
        }
    };
    let report = match schema.as_deref() {
        Some(schema) => recite_compiler::extract_pot_with_schema(inputs, schema),
        None => recite_compiler::extract_pot(inputs),
    };
    extract_report(report, args.output.as_deref())
}

enum SchemaLoad {
    Loaded(Option<Box<ProjectSchema>>),
    Diagnostics(Vec<recite_core::DiagnosticRecord>),
}

fn schema_or_diagnostics(schema_path: Option<&Path>) -> Result<SchemaLoad, CliError> {
    let Some(schema_path) = schema_path else {
        return Ok(SchemaLoad::Loaded(None));
    };
    let loaded = load_schema(schema_path)?;
    if loaded.diagnostics.is_empty() {
        return Ok(SchemaLoad::Loaded(loaded.schema.map(Box::new)));
    }
    Ok(SchemaLoad::Diagnostics(diagnostic_records(
        &loaded.diagnostics,
    )?))
}

fn extract_report(
    report: PotExtractionReport,
    output: Option<&Path>,
) -> Result<StructuredOutcome, CliError> {
    let diagnostics = diagnostic_records(&report.diagnostics)?;
    let Some(catalog) = report.catalog else {
        return Ok(StructuredOutcome::content_diagnostics(
            ContentDiagnosticData::Extract { diagnostics },
        ));
    };

    if let Some(output) = output {
        write_artifact(output, catalog.to_pot_string().as_bytes())?;
        let artifact = artifact_metadata(output)?;
        return Ok(StructuredOutcome::success(SuccessData::ExtractArtifact {
            diagnostics,
            artifact,
        }));
    }

    let entries = catalog
        .entries
        .iter()
        .map(CatalogEntry::from)
        .collect::<Vec<_>>();
    Ok(StructuredOutcome::success(SuccessData::ExtractEntries {
        diagnostics,
        entries,
    }))
}

fn runtime(
    args: RuntimeArgs,
    options: RuntimeFixtureOptions,
) -> Result<StructuredOutcome, CliError> {
    let asset = load_compiled_asset(&args.asset)?;
    let fixture = load_runtime_fixture(&args.fixture)?;
    let dialogue_preview = dialogue_preview_from_fixture(&fixture)?
        .map(crate::dialogue_locale::LoadedDialoguePreview::load)
        .transpose()?;
    let execution = execute_runtime_fixture(
        &asset,
        &args.block,
        &fixture,
        dialogue_preview
            .as_ref()
            .map(crate::dialogue_locale::LoadedDialoguePreview::traversal_preview),
        dialogue_preview
            .as_ref()
            .map(crate::dialogue_locale::LoadedDialoguePreview::locale_fallbacks),
        options,
        &crate::default_messages(),
    )?;
    Ok(StructuredOutcome::success(SuccessData::Runtime {
        trace: trace_document(execution),
    }))
}

fn write_artifact(output: &Path, contents: &[u8]) -> Result<(), CliError> {
    write_staged(output, contents).map_err(|error| match error {
        CliError::Write { source, .. } => CliError::Write {
            path: output.to_owned(),
            source,
        },
        error => error,
    })
}
